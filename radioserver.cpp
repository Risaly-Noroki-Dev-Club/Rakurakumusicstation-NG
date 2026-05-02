#include <crow_all.h>
#include <hiredis.h>
#include <sys/epoll.h>
#include <atomic>
#include <mutex>
#include <thread>
#include <fstream>
#include <vector>
#include <deque>
#include <sys/socket.h>
#include <sys/wait.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <fcntl.h>
#include <filesystem>
#include <memory>
#include <condition_variable>
#include <csignal>
#include <cstring>
#include <poll.h>
#include <unordered_map>
#include <iostream>
#include <algorithm>
#include <set>
#include <sstream>
#include <iomanip>
#include <regex>
#include <random>
#include <chrono>
#include "metadata.hpp"

namespace fs = std::filesystem;

// =============================================================================
// 配置常量
// =============================================================================
namespace Config {
    constexpr int WEB_PORT = 2240;
    constexpr int REDIS_PORT = 6379;
    const char* const REDIS_HOST = "127.0.0.1";
    constexpr size_t BUFFER_CAPACITY = 512 * 1024;
    constexpr size_t AUDIO_CHUNK_SIZE = 16384;
    constexpr int EPOLL_TIMEOUT_MS = 100;
    constexpr int POLL_TIMEOUT_MS = 200;
    constexpr int MAX_EVENTS = 1024;
    constexpr int MAX_CONNECTIONS = 1024;
    constexpr int STATE_PUBLISH_MS = 500;
    constexpr int CROSSFADE_SECONDS = 3;
    const std::vector<std::string> SUPPORTED_FORMATS = {
        ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"
    };
}

// =============================================================================
// 简单的 JSON 辅助函数（避免引入额外依赖）
// =============================================================================
static std::string json_escape(const std::string& s) {
    std::string out;
    out.reserve(s.size() + 8);
    for (char c : s) {
        switch (c) {
            case '"':  out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default:   out += c;
        }
    }
    return out;
}

// =============================================================================
// 线程安全的环形缓冲区（BroadcastBuffer）
// =============================================================================
class BroadcastBuffer {
public:
    explicit BroadcastBuffer(size_t capacity = Config::BUFFER_CAPACITY)
        : capacity_(capacity), mask_(capacity - 1), buffer_(capacity) {
        if (capacity == 0 || (capacity & (capacity - 1)) != 0) {
            throw std::runtime_error("Capacity must be power of two");
        }
    }

    void push(const char* data, size_t len) {
        if (len == 0 || len > capacity_) return;
        std::lock_guard<std::mutex> lock(write_mutex_);
        size_t current_rp = consume_pos_.load(std::memory_order_relaxed);
        size_t current_wp = write_pos_.load(std::memory_order_relaxed);
        size_t used = (current_wp >= current_rp) ?
                     (current_wp - current_rp) :
                     (capacity_ - current_rp + current_wp);
        size_t free = capacity_ - used;
        if (len > free) {
            consume_pos_.store((current_rp + (len - free)) & mask_,
                               std::memory_order_relaxed);
        }
        size_t new_wp = (current_wp + len) & mask_;
        size_t first_seg = std::min(len, capacity_ - (current_wp & mask_));
        std::memcpy(&buffer_[current_wp & mask_], data, first_seg);
        if (first_seg < len) {
            std::memcpy(&buffer_[0], data + first_seg, len - first_seg);
        }
        write_pos_.store(new_wp, std::memory_order_release);
        data_cv_.notify_all();
    }

    size_t read(size_t& consume_pos, char* dest, size_t max_len) {
        size_t wp = write_pos_.load(std::memory_order_acquire);
        size_t rp = consume_pos;
        if (rp == wp) return 0;
        size_t avail = (wp > rp) ? (wp - rp) : (capacity_ - rp + wp);
        size_t to_read = std::min(avail, max_len);
        size_t first_seg = std::min(to_read, capacity_ - (rp & mask_));
        std::memcpy(dest, &buffer_[rp & mask_], first_seg);
        if (first_seg < to_read) {
            std::memcpy(dest + first_seg, &buffer_[0], to_read - first_seg);
        }
        consume_pos = (rp + to_read) & mask_;
        return to_read;
    }

    bool wait_for_data(size_t& consume_pos, int timeout_ms = 100) {
        std::unique_lock<std::mutex> lock(cv_mutex_);
        size_t wp = write_pos_.load(std::memory_order_acquire);
        if (consume_pos != wp) return true;
        return data_cv_.wait_for(lock, std::chrono::milliseconds(timeout_ms),
            [this, &consume_pos]() {
                return consume_pos != write_pos_.load(std::memory_order_acquire);
            });
    }

    size_t current_write_pos() const {
        return write_pos_.load(std::memory_order_acquire);
    }

    void wakeup_all() {
        std::lock_guard<std::mutex> lock(cv_mutex_);
        data_cv_.notify_all();
    }

private:
    const size_t capacity_;
    const size_t mask_;
    std::vector<char> buffer_;
    std::atomic<size_t> write_pos_{0};
    std::atomic<size_t> consume_pos_{0};
    std::mutex write_mutex_;
    std::mutex cv_mutex_;
    std::condition_variable data_cv_;
};

// =============================================================================
// 客户端连接管理（ClientConnection + StreamServer）
// =============================================================================
class ClientConnection {
public:
    ClientConnection(int fd, BroadcastBuffer* buffer)
        : fd_(fd), buffer_(buffer), consume_pos_(0), shutdown_(false) {
        int flags = fcntl(fd_, F_GETFL, 0);
        if (flags >= 0) fcntl(fd_, F_SETFL, flags | O_NONBLOCK);
        int yes = 1;
        setsockopt(fd_, IPPROTO_TCP, TCP_NODELAY, &yes, sizeof(yes));
        struct timeval tv{1, 0};
        setsockopt(fd_, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
    }

    ~ClientConnection() { close_socket(); }

    void close_socket() {
        if (!shutdown_.exchange(true)) {
            if (fd_ >= 0) {
                ::shutdown(fd_, SHUT_RDWR);
                ::close(fd_);
                fd_ = -1;
            }
        }
    }

    int fd() const { return fd_; }
    bool is_shutdown() const { return shutdown_; }

    bool send_header() {
        if (shutdown_ || header_sent_) return true;
        const std::string header =
            "HTTP/1.1 200 OK\r\n"
            "Content-Type: audio/mpeg\r\n"
            "Connection: keep-alive\r\n"
            "Cache-Control: no-cache, no-store\r\n"
            "Pragma: no-cache\r\n"
            "Server: Rakuraku-Radio\r\n"
            "Access-Control-Allow-Origin: *\r\n"
            "\r\n";
        ssize_t sent = ::send(fd_, header.c_str(), header.size(), MSG_NOSIGNAL);
        if (sent > 0) {
            header_sent_ = true;
            return true;
        }
        return (sent < 0 && (errno == EAGAIN || errno == EWOULDBLOCK));
    }

    bool send_audio() {
        if (shutdown_) return false;
        char tmp[Config::AUDIO_CHUNK_SIZE];
        size_t read_bytes = buffer_->read(consume_pos_, tmp, sizeof(tmp));
        if (read_bytes == 0) return true;
        ssize_t sent = ::send(fd_, tmp, read_bytes, MSG_NOSIGNAL);
        return sent >= 0 || errno == EAGAIN || errno == EWOULDBLOCK;
    }

private:
    int fd_;
    BroadcastBuffer* buffer_;
    size_t consume_pos_;
    std::atomic<bool> shutdown_{false};
    bool header_sent_{false};
};

class StreamServer {
public:
    StreamServer(BroadcastBuffer* buffer) : buffer_(buffer) {}

    ~StreamServer() { stop(); }

    void add_client(int fd) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (clients_.size() >= Config::MAX_CONNECTIONS) {
            close(fd);
            return;
        }
        clients_.emplace_back(std::make_unique<ClientConnection>(fd, buffer_));
        // 标记需要将新客户端加入 epoll
        new_clients_.push_back(clients_.back().get());
    }

    bool start() {
        if (running_) return false;
        epoll_fd_ = epoll_create1(0);
        if (epoll_fd_ < 0) { std::cerr << "[Stream] epoll_create1 failed" << std::endl; return false; }
        running_ = true;
        thread_ = std::thread(&StreamServer::worker_loop, this);
        std::cout << "[Stream] Server started" << std::endl;
        return true;
    }

    void stop() {
        if (!running_.exchange(false)) return;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            for (auto& c : clients_) c->close_socket();
        }
        buffer_->wakeup_all();
        if (thread_.joinable()) thread_.join();
        if (epoll_fd_ >= 0) { close(epoll_fd_); epoll_fd_ = -1; }
        std::cout << "[Stream] Server stopped" << std::endl;
    }

    size_t client_count() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return clients_.size();
    }

private:
    BroadcastBuffer* buffer_;
    int epoll_fd_{-1};
    std::atomic<bool> running_{false};
    std::thread thread_;
    mutable std::mutex mutex_;
    std::deque<std::unique_ptr<ClientConnection>> clients_;
    std::vector<ClientConnection*> new_clients_;

    void worker_loop() {
        struct epoll_event ev, events[Config::MAX_EVENTS];
        while (running_) {
            // 注册新客户端到 epoll
            {
                std::lock_guard<std::mutex> lock(mutex_);
                for (auto* c : new_clients_) {
                    ev.events = EPOLLIN | EPOLLRDHUP | EPOLLET;
                    ev.data.ptr = c;
                    if (epoll_ctl(epoll_fd_, EPOLL_CTL_ADD, c->fd(), &ev) < 0) {
                        c->close_socket();
                    }
                }
                new_clients_.clear();
            }

            int nfds = epoll_wait(epoll_fd_, events, Config::MAX_EVENTS, Config::EPOLL_TIMEOUT_MS);
            if (nfds < 0) {
                if (errno == EINTR) continue;
                break;
            }

            for (int i = 0; i < nfds; ++i) {
                auto* conn = static_cast<ClientConnection*>(events[i].data.ptr);
                if (events[i].events & (EPOLLERR | EPOLLHUP | EPOLLRDHUP)) {
                    conn->close_socket();
                    continue;
                }
                if (!conn->send_header()) {
                    conn->close_socket();
                    continue;
                }
            }

            // 对所有已发送头部的客户端推送音频数据
            {
                std::lock_guard<std::mutex> lock(mutex_);
                for (auto& c : clients_) {
                    if (!c->is_shutdown() && !c->send_audio()) {
                        c->close_socket();
                    }
                }
                // 清理已关闭的客户端
                clients_.erase(
                    std::remove_if(clients_.begin(), clients_.end(),
                        [](const auto& c) { return c->is_shutdown(); }),
                    clients_.end());
            }
        }
    }
};

// =============================================================================
// Redis pub/sub 桥接层（RedisBridge）
// =============================================================================
class RedisBridge {
public:
    RedisBridge(const char* host = Config::REDIS_HOST, int port = Config::REDIS_PORT)
        : host_(host), port_(port) {}

    ~RedisBridge() { stop(); }

    bool connect() {
        pub_ctx_ = redisConnect(host_, port_);
        if (!pub_ctx_ || pub_ctx_->err) {
            std::cerr << "[Redis] Publish connection failed: "
                      << (pub_ctx_ ? pub_ctx_->errstr : "OOM") << std::endl;
            return false;
        }
        std::cout << "[Redis] Publish connection established to "
                  << host_ << ":" << port_ << std::endl;
        return true;
    }

    void publish_state(const std::string& json) {
        if (!pub_ctx_) return;
        auto* reply = static_cast<redisReply*>(
            redisCommand(pub_ctx_, "PUBLISH playback_state %s", json.c_str()));
        if (reply) freeReplyObject(reply);
    }

    void publish_queue_event(const std::string& json) {
        if (!pub_ctx_) return;
        auto* reply = static_cast<redisReply*>(
            redisCommand(pub_ctx_, "PUBLISH queue_event %s", json.c_str()));
        if (reply) freeReplyObject(reply);
    }

    void start_subscriber() {
        running_ = true;
        sub_thread_ = std::thread([this]() {
            while (running_) {
                auto* ctx = redisConnect(host_, port_);
                if (!ctx || ctx->err) {
                    std::cerr << "[Redis] Subscriber connection failed: "
                              << (ctx ? ctx->errstr : "OOM") << ", retrying in 3s..." << std::endl;
                    if (ctx) redisFree(ctx);
                    std::this_thread::sleep_for(std::chrono::seconds(3));
                    continue;
                }
                std::cout << "[Redis] Subscriber connected, subscribing to command channel" << std::endl;

                auto* sub_reply = static_cast<redisReply*>(
                    redisCommand(ctx, "SUBSCRIBE command"));
                if (sub_reply) freeReplyObject(sub_reply);

                while (running_) {
                    redisReply* reply = nullptr;
                    if (redisGetReply(ctx, reinterpret_cast<void**>(&reply)) != REDIS_OK) {
                        std::cerr << "[Redis] Subscriber read error, reconnecting..." << std::endl;
                        break;
                    }
                    if (!reply) break;

                    if (reply->type == REDIS_REPLY_ARRAY && reply->elements == 3) {
                        // message: [ "message", "command", "payload" ]
                        std::string channel(reply->element[1]->str, reply->element[1]->len);
                        std::string payload(reply->element[2]->str, reply->element[2]->len);
                        if (channel == "command") {
                            std::lock_guard<std::mutex> lock(cmd_mutex_);
                            pending_commands_.push_back(payload);
                        }
                    }
                    freeReplyObject(reply);
                }
                redisFree(ctx);
                if (running_) std::this_thread::sleep_for(std::chrono::seconds(3));
            }
        });
    }

    void stop() {
        running_ = false;
        if (sub_thread_.joinable()) sub_thread_.join();
        if (pub_ctx_) { redisFree(pub_ctx_); pub_ctx_ = nullptr; }
    }

    bool poll_command(std::string& out) {
        std::lock_guard<std::mutex> lock(cmd_mutex_);
        if (pending_commands_.empty()) return false;
        out = pending_commands_.front();
        pending_commands_.erase(pending_commands_.begin());
        return true;
    }

    bool is_running() const { return running_; }

private:
    const char* host_;
    int port_;
    redisContext* pub_ctx_{nullptr};
    std::thread sub_thread_;
    std::atomic<bool> running_{false};
    std::mutex cmd_mutex_;
    std::vector<std::string> pending_commands_;
};

// =============================================================================
// 构建 PlaybackState JSON（与 Rust 端 models::PlaybackState 格式一致）
// =============================================================================
static std::string build_playback_state_json(
    int64_t song_index,
    const std::string& file_path,
    int64_t position_ms,
    int64_t duration_ms,
    const std::string& status,
    uint64_t total_bytes_sent)
{
    auto now_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        std::chrono::system_clock::now().time_since_epoch()).count();
    std::ostringstream ss;
    ss << "{"
       << "\"song_id\":" << song_index << ","
       << "\"file_path\":\"" << json_escape(file_path) << "\","
       << "\"position_ms\":" << position_ms << ","
       << "\"duration_ms\":" << duration_ms << ","
       << "\"lyrics_line\":null,"
       << "\"status\":\"" << status << "\","
       << "\"total_bytes_sent\":" << total_bytes_sent << ","
       << "\"bitrate_kbps\":128,"
       << "\"track_start_timestamp_ms\":" << now_ms
       << "}";
    return ss.str();
}

// 解析 AudioCommand JSON（Rust → C++）
static bool parse_audio_command(const std::string& json_str,
                                std::string& cmd_type,
                                std::string& file_path)
{
    // 简易 JSON 解析：提取 "type" 和 "file_path" 字段
    auto extract_str = [&](const std::string& key) -> std::string {
        std::string search = "\"" + key + "\":\"";
        size_t pos = json_str.find(search);
        if (pos == std::string::npos) {
            search = "\"" + key + "\": \"";
            pos = json_str.find(search);
        }
        if (pos == std::string::npos) return "";
        pos += search.size();
        size_t end = json_str.find('"', pos);
        if (end == std::string::npos) return "";
        return json_str.substr(pos, end - pos);
    };
    cmd_type = extract_str("type");
    file_path = extract_str("file_path");
    return !cmd_type.empty();
}

// =============================================================================
// 音频播放器（AudioPlayer）
// =============================================================================
class AudioPlayer {
public:
    AudioPlayer(BroadcastBuffer* buffer, std::vector<std::string>* playlist,
            std::atomic<size_t>* current_track, std::mutex* playlist_mutex,
            std::vector<TrackMetadata>* playlist_metadata,
            RedisBridge* redis = nullptr)
    : buffer_(buffer), playlist_(playlist), current_track_(current_track),
      playlist_mutex_(playlist_mutex), playlist_metadata_(playlist_metadata),
      redis_(redis), running_(false), skip_track_(false) {}

    ~AudioPlayer() { stop(); }

    bool start() {
        if (running_) return false;
        signal(SIGPIPE, SIG_IGN);
        running_ = true;
        thread_ = std::thread(&AudioPlayer::worker_loop, this);
        std::cout << "[Audio] Player started" << std::endl;
        return true;
    }

    void stop() {
        if (!running_.exchange(false)) return;
        skip_track_ = true;
        cleanup_preload();
        {
            std::lock_guard<std::mutex> lock(idle_mutex_);
            idle_cv_.notify_all();
        }
        if (thread_.joinable()) thread_.join();
        // 发送停止状态
        if (redis_) {
            redis_->publish_state(
                build_playback_state_json(0, "", 0, 0, "stopped", 0));
        }
        std::cout << "[Audio] Player stopped" << std::endl;
    }

    void skip_current_track() { skip_track_ = true; }

    void play_file(const std::string& file_path) {
        std::lock_guard<std::mutex> lock(*playlist_mutex_);
        for (size_t i = 0; i < playlist_->size(); ++i) {
            if ((*playlist_)[i] == file_path) {
                current_track_->store(i);
                skip_track_ = true;
                return;
            }
        }
        // 文件不在播放列表中，添加到列表并播放
        playlist_->push_back(file_path);
        playlist_metadata_->push_back(TrackMetadata("./media/" + file_path));
        current_track_->store(playlist_->size() - 1);
        skip_track_ = true;
    }

private:
    BroadcastBuffer* buffer_;
    std::vector<std::string>* playlist_;
    std::atomic<size_t>* current_track_;
    std::mutex* playlist_mutex_;
    std::vector<TrackMetadata>* playlist_metadata_;
    RedisBridge* redis_;
    std::atomic<bool> running_{false};
    std::atomic<bool> skip_track_{false};
    std::thread thread_;
    std::mutex idle_mutex_;
    std::condition_variable idle_cv_;
    std::atomic<uint64_t> total_bytes_sent_{0};
    int64_t current_duration_ms_{0};
    int64_t track_start_epoch_ms_{0};

    // 平滑过渡（crossfade）预加载
    pid_t preload_child_pid_ = -1;
    int preload_pipe_fd_ = -1;
    std::vector<uint8_t> preload_buffer_;
    size_t preload_track_idx_ = 0;
    std::string preload_filename_;

    void interruptible_wait(std::chrono::milliseconds dur) {
        std::unique_lock<std::mutex> lock(idle_mutex_);
        idle_cv_.wait_for(lock, dur, [this]() { return !running_.load(); });
    }

    // 构建 ffmpeg 命令行参数（带 afade 滤镜）
    void build_ffmpeg_argv(const std::string& filename, int64_t duration_ms,
                           bool is_preload,
                           std::vector<std::string>& out_strs,
                           std::vector<const char*>& out_argv) {
        out_strs.clear();
        out_strs = {"ffmpeg", "-nostdin", "-re", "-loglevel", "error"};
        out_strs.insert(out_strs.end(), {"-i", filename, "-vn",
                         "-c:a", "libmp3lame", "-b:a", "128k",
                         "-ar", "44100", "-ac", "2"});

        // 构建 afade 滤镜
        std::string afade;
        if (is_preload) {
            afade = "afade=t=in:d=" + std::to_string(Config::CROSSFADE_SECONDS) +
                    ":curve=tri";
        } else if (duration_ms > Config::CROSSFADE_SECONDS * 2 * 1000) {
            double st = (duration_ms / 1000.0) - Config::CROSSFADE_SECONDS;
            std::ostringstream oss;
            oss << "afade=t=out:st=" << std::fixed << std::setprecision(2) << st;
            oss << ":d=" << Config::CROSSFADE_SECONDS << ":curve=tri";
            afade = oss.str();
        }

        if (!afade.empty()) {
            out_strs.push_back("-af");
            out_strs.push_back(afade);
        }

        out_strs.insert(out_strs.end(), {"-f", "mp3", "pipe:1"});

        out_argv.clear();
        for (const auto& s : out_strs) out_argv.push_back(s.c_str());
        out_argv.push_back(nullptr);
    }

    // 启动预加载：fork ffmpeg 解码下一首的前几秒（含 fade-in）
    void start_preload(const std::string& filename, size_t track_idx) {
        preload_filename_ = filename;
        preload_track_idx_ = track_idx;
        preload_buffer_.clear();

        int fds[2];
        if (pipe2(fds, O_CLOEXEC) != 0) {
            std::cerr << "[XFade] preload pipe2 failed: " << strerror(errno) << std::endl;
            return;
        }

        pid_t pid = fork();
        if (pid < 0) {
            std::cerr << "[XFade] preload fork failed: " << strerror(errno) << std::endl;
            close(fds[0]); close(fds[1]);
            return;
        }

        if (pid == 0) {
            if (dup2(fds[1], STDOUT_FILENO) < 0) _exit(127);
            close(fds[0]);
            std::vector<std::string> strs;
            std::vector<const char*> argv;
            build_ffmpeg_argv(filename, 0, true, strs, argv);
            execvp("ffmpeg", const_cast<char* const*>(argv.data()));
            _exit(127);
        }

        close(fds[1]);
        preload_pipe_fd_ = fds[0];
        preload_child_pid_ = pid;

        // 设为非阻塞模式，在主循环中读取
        int flags = fcntl(preload_pipe_fd_, F_GETFL, 0);
        if (flags >= 0) fcntl(preload_pipe_fd_, F_SETFL, flags | O_NONBLOCK);

        std::cout << "[XFade] Preloading next track: " << filename << std::endl;
    }

    // 从预加载管道读取数据（非阻塞）
    void read_preload() {
        if (preload_pipe_fd_ < 0) return;
        char buf[Config::AUDIO_CHUNK_SIZE];
        ssize_t n;
        do {
            n = read(preload_pipe_fd_, buf, sizeof(buf));
        } while (n < 0 && errno == EINTR);
        if (n > 0) {
            preload_buffer_.insert(preload_buffer_.end(), buf, buf + n);
        }
    }

    // 排出预加载缓冲区到 BroadcastBuffer，然后切换为主播放管道
    void drain_and_switch(int& pipe_fd, pid_t& child_pid,
                          size_t& track_idx, std::string& filename) {
        // 排出预加载缓冲（track B 的 fade-in 部分）
        if (!preload_buffer_.empty()) {
            buffer_->push(reinterpret_cast<const char*>(preload_buffer_.data()), preload_buffer_.size());
            total_bytes_sent_ += preload_buffer_.size();
            std::cout << "[XFade] Drained " << preload_buffer_.size()
                      << " bytes of preloaded audio" << std::endl;
            preload_buffer_.clear();
        }

        // 切换到预加载的管道，继续播放
        pipe_fd = preload_pipe_fd_;
        child_pid = preload_child_pid_;
        track_idx = preload_track_idx_;
        filename = preload_filename_;

        // 恢复阻塞模式
        int flags = fcntl(pipe_fd, F_GETFL, 0);
        if (flags >= 0) fcntl(pipe_fd, F_SETFL, flags & ~O_NONBLOCK);

        preload_pipe_fd_ = -1;
        preload_child_pid_ = -1;

        std::cout << "[XFade] Switched to next track" << std::endl;
    }

    // 清理预加载状态（取消/跳过时调用）
    void cleanup_preload() {
        if (preload_pipe_fd_ >= 0) { close(preload_pipe_fd_); preload_pipe_fd_ = -1; }
        if (preload_child_pid_ > 0) {
            kill(preload_child_pid_, SIGTERM);
            waitpid(preload_child_pid_, nullptr, 0);
            preload_child_pid_ = -1;
        }
        preload_buffer_.clear();
    }

    void worker_loop() {
        while (running_) {
            bool playlist_empty = false;
            {
                std::lock_guard<std::mutex> lock(*playlist_mutex_);
                playlist_empty = playlist_->empty();
            }
            if (playlist_empty) {
                interruptible_wait(std::chrono::seconds(5));
                continue;
            }
            play_next_track();
        }
    }

    void play_next_track() {
        pid_t child_pid = -1;
        int pipe_fd = -1;
        size_t playlist_size = 0;

        std::string filename;
        size_t track_idx = 0;

    track_setup:
        {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            playlist_size = playlist_->size();
            if (playlist_size == 0) return;
            track_idx = current_track_->load() % playlist_size;
            filename = "./media/" + playlist_->at(track_idx);

            if (track_idx < playlist_metadata_->size()) {
                current_duration_ms_ = static_cast<int64_t>(
                    playlist_metadata_->at(track_idx).duration * 1000.0);
            } else {
                current_duration_ms_ = 0;
            }

            std::cout << "[Audio] Playing: " << playlist_->at(track_idx)
                      << " (" << track_idx + 1 << "/" << playlist_->size()
                      << ")" << (current_duration_ms_ > 0
                          ? " [" + std::to_string(current_duration_ms_ / 1000) + "s]"
                          : " [duration unknown]")
                      << std::endl;
        }

        if (!fs::exists(filename)) {
            std::cerr << "[Audio] File not found: " << filename << std::endl;
            std::cerr << "[Audio] CWD: " << fs::current_path() << std::endl;
            (*current_track_)++;
            interruptible_wait(std::chrono::seconds(1));
            return;
        }

        // ffmpeg 子进程：编码为 MP3 流（带 afade 滤镜）
        int fds[2];
        if (pipe2(fds, O_CLOEXEC) != 0) {
            std::cerr << "[Audio] pipe2 failed: " << strerror(errno) << std::endl;
            (*current_track_)++;
            interruptible_wait(std::chrono::seconds(1));
            return;
        }

        child_pid = fork();
        if (child_pid < 0) {
            std::cerr << "[Audio] fork failed: " << strerror(errno) << std::endl;
            close(fds[0]); close(fds[1]);
            (*current_track_)++;
            interruptible_wait(std::chrono::seconds(1));
            return;
        }

        if (child_pid == 0) {
            if (dup2(fds[1], STDOUT_FILENO) < 0) _exit(127);
            close(fds[0]);
            std::vector<std::string> argv_strs;
            std::vector<const char*> argv_ptrs;
            build_ffmpeg_argv(filename, current_duration_ms_, false, argv_strs, argv_ptrs);
            execvp("ffmpeg", const_cast<char* const*>(argv_ptrs.data()));
            _exit(127);
        }

        close(fds[1]);
        pipe_fd = fds[0];

    stream_loop:
        struct pollfd pfd = {pipe_fd, POLLIN, 0};
        char buffer[Config::AUDIO_CHUNK_SIZE];
        skip_track_ = false;
        total_bytes_sent_ = 0;
        track_start_epoch_ms_ = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now().time_since_epoch()).count();

        bool error_occurred = false;
        auto last_state_publish = std::chrono::steady_clock::now();
        bool preload_triggered = false;

        while (!skip_track_ && running_ && !error_occurred) {
            int ret = poll(&pfd, 1, Config::POLL_TIMEOUT_MS);

            if (ret > 0) {
                if (pfd.revents & POLLIN) {
                    ssize_t bytes;
                    do {
                        bytes = read(pipe_fd, buffer, sizeof(buffer));
                    } while (bytes < 0 && errno == EINTR);

                    if (bytes > 0) {
                        buffer_->push(buffer, bytes);
                        total_bytes_sent_ += bytes;
                    } else if (bytes == 0) {
                        break;
                    } else if (bytes < 0) {
                        if (errno != EAGAIN && errno != EWOULDBLOCK) {
                            std::cerr << "[Audio] Read error: " << strerror(errno) << std::endl;
                            error_occurred = true;
                        }
                    }
                } else if (pfd.revents & POLLHUP) {
                    break;
                } else if (pfd.revents & (POLLERR | POLLNVAL)) {
                    std::cerr << "[Audio] Pipe error: revents=" << pfd.revents << std::endl;
                    error_occurred = true;
                }
            } else if (ret < 0) {
                if (errno == EINTR) continue;
                std::cerr << "[Audio] Poll error: " << strerror(errno) << std::endl;
                error_occurred = true;
            }

            // 交叉淡入淡出：在曲目结束前 CROSSFADE_SECONDS 秒启动预加载
            if (!preload_triggered && !skip_track_ && running_
                && current_duration_ms_ > Config::CROSSFADE_SECONDS * 1000
                && preload_child_pid_ < 0) {
                auto now = std::chrono::steady_clock::now();
                int64_t elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                    now.time_since_epoch()).count() - track_start_epoch_ms_;
                if (elapsed_ms >= current_duration_ms_ - (Config::CROSSFADE_SECONDS * 1000)) {
                    size_t next_idx = 0;
                    std::string next_fn;
                    {
                        std::lock_guard<std::mutex> lock(*playlist_mutex_);
                        size_t sz = playlist_->size();
                        if (sz > 0) {
                            next_idx = (current_track_->load() + 1) % sz;
                            next_fn = "./media/" + playlist_->at(next_idx);
                        }
                    }
                    if (!next_fn.empty() && fs::exists(next_fn)) {
                        start_preload(next_fn, next_idx);
                        preload_triggered = true;
                    }
                }
            }

            // 从预加载管道读取数据（非阻塞）
            if (preload_pipe_fd_ >= 0) {
                read_preload();
            }

            // 定期发布播放状态到 Redis
            if (redis_) {
                auto now = std::chrono::steady_clock::now();
                if (now - last_state_publish >= std::chrono::milliseconds(Config::STATE_PUBLISH_MS)) {
                    last_state_publish = now;
                    auto position_ms = now.time_since_epoch().count() / 1000000 - track_start_epoch_ms_;
                    if (current_duration_ms_ > 0 && position_ms > current_duration_ms_) {
                        position_ms = current_duration_ms_;
                    }
                    const char* status = preload_triggered ? "crossfading" : "playing";
                    redis_->publish_state(
                        build_playback_state_json(
                            static_cast<int64_t>(track_idx),
                            playlist_->at(track_idx),
                            position_ms,
                            current_duration_ms_,
                            status,
                            total_bytes_sent_.load()));
                }
            }

            // 处理从 Redis 收到的命令
            if (redis_) {
                std::string cmd_json;
                while (redis_->poll_command(cmd_json)) {
                    std::string cmd_type, cmd_file;
                    if (parse_audio_command(cmd_json, cmd_type, cmd_file)) {
                        if ((cmd_type == "skip" || cmd_type == "next") && running_ && playlist_size > 0) {
                            std::lock_guard<std::mutex> lock(*playlist_mutex_);
                            size_t sz = playlist_->size();
                            if (sz > 0) {
                                current_track_->store((current_track_->load() + 1) % sz);
                                skip_track_ = true;
                                std::cout << "[Redis] Received skip command" << std::endl;
                            }
                        } else if (cmd_type == "prev" && running_ && playlist_size > 0) {
                            std::lock_guard<std::mutex> lock(*playlist_mutex_);
                            size_t sz = playlist_->size();
                            if (sz > 0) {
                                current_track_->store((current_track_->load() + sz - 1) % sz);
                                skip_track_ = true;
                                std::cout << "[Redis] Received prev command" << std::endl;
                            }
                        } else if (cmd_type == "play" && !cmd_file.empty()) {
                            play_file(cmd_file);
                            std::cout << "[Redis] Received play command: " << cmd_file << std::endl;
                        } else if (cmd_type == "stop") {
                            skip_track_ = true;
                            std::cout << "[Redis] Received stop command" << std::endl;
                        }
                    }
                }
            }
        }

        // 回收 ffmpeg 子进程
        if (pipe_fd >= 0) { close(pipe_fd); pipe_fd = -1; }
        if (child_pid > 0) {
            kill(child_pid, SIGTERM);
            int status = 0;
            int waited = 0;
            const int max_wait_ms = 2000;
            while (waited < max_wait_ms) {
                pid_t w = waitpid(child_pid, &status, WNOHANG);
                if (w > 0) break;
                if (w < 0 && errno != EINTR) break;
                std::this_thread::sleep_for(std::chrono::milliseconds(50));
                waited += 50;
            }
            if (waited >= max_wait_ms) {
                kill(child_pid, SIGKILL);
                while (waitpid(child_pid, &status, 0) < 0) {
                    if (errno != EINTR) break;
                }
            }
            child_pid = -1;
        }

        // 交叉淡入淡出：如果预加载已完成，排出预加载缓冲并切换到下一首
        if (!skip_track_ && running_ && preload_child_pid_ > 0) {
            drain_and_switch(pipe_fd, child_pid, track_idx, filename);
            {
                std::lock_guard<std::mutex> lock(*playlist_mutex_);
                if (track_idx < playlist_metadata_->size()) {
                    current_duration_ms_ = static_cast<int64_t>(
                        playlist_metadata_->at(track_idx).duration * 1000.0);
                }
                current_track_->store(track_idx);
                playlist_size = playlist_->size();
            }
            {
                std::string fn;
                {
                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    if (track_idx < playlist_->size())
                        fn = playlist_->at(track_idx);
                }
                std::cout << "[Audio] Crossfaded to: " << (fn.empty() ? filename : fn)
                          << " (" << track_idx + 1 << "/" << playlist_size << ")"
                          << " [" << (current_duration_ms_ / 1000) << "s]" << std::endl;
            }
            goto stream_loop;
        }

        // 清理任何残留的预加载状态（被 skip/stop 中断时）
        cleanup_preload();

        // 自动切换到下一首
        if (!skip_track_ && running_) {
            current_track_->store((current_track_->load() + 1) % (playlist_size > 0 ? playlist_size : 1));
        }
        skip_track_ = false;
    }
};

// =============================================================================
// Web 服务器（精简版：仅 /stream 音频流 + /health 健康检查）
// =============================================================================
class WebServer {
public:
    WebServer(StreamServer* stream_server)
        : stream_server_(stream_server) {}

    ~WebServer() { stop(); }

    bool start() {
        if (running_) return false;
        setup_routes();
        running_ = true;
        thread_ = std::thread([this]() {
            try {
                std::cout << "[Web] Streaming server on port " << Config::WEB_PORT << std::endl;
                app_.signal_clear();
                app_.port(Config::WEB_PORT).multithreaded().run();
            } catch (const std::exception& e) {
                std::cerr << "[Web] Error: " << e.what() << std::endl;
            }
            running_ = false;
        });
        return true;
    }

    void stop() {
        if (!running_.exchange(false)) return;
        app_.stop();
        if (thread_.joinable()) thread_.join();
        std::cout << "[Web] Server stopped" << std::endl;
    }

private:
    crow::SimpleApp app_;
    StreamServer* stream_server_;
    std::thread thread_;
    std::atomic<bool> running_{false};

    void setup_routes() {
        // 音频流端点
        CROW_ROUTE(app_, "/stream")([this](const crow::request&, crow::response& res) {
            if (!res.get_socket_fd_helper_) {
                res.code = 500;
                res.end("stream unavailable");
                return;
            }
            int crow_fd = res.get_socket_fd_helper_();
            if (crow_fd < 0) {
                res.code = 500;
                res.end("invalid socket");
                std::cerr << "[Stream] get_socket_fd_helper_ returned invalid fd: "
                          << crow_fd << std::endl;
                return;
            }
            int fd = ::dup(crow_fd);
            if (fd < 0) {
                res.code = 500;
                res.end("dup failed");
                std::cerr << "[Stream] dup(" << crow_fd << ") failed: "
                          << strerror(errno) << " (errno=" << errno << ")" << std::endl;
                return;
            }
            res.take_over();
            stream_server_->add_client(fd);
        });

        // 健康检查端点
        CROW_ROUTE(app_, "/health")([this]() {
            crow::json::wvalue result;
            result["status"] = "ok";
            result["service"] = "rakuraku-audio-engine";
            result["clients"] = static_cast<int>(stream_server_->client_count());
            return crow::response(result);
        });
    }
};

// =============================================================================
// RadioServer 主控制器
// =============================================================================
class RadioServer {
public:
    RadioServer() { init_playlist(); }

    ~RadioServer() { stop(); }

    bool start() {
        if (running_) return false;
        std::cout << "\n"
            "╔══════════════════════════════════════════╗\n"
            "║     Rakuraku Audio Engine 启动中...     ║\n"
            "╚══════════════════════════════════════════╝\n" << std::endl;

        // 尝试连接 Redis（可选，失败不阻止启动）
        if (redis_.connect()) {
            redis_connected_ = true;
            redis_.start_subscriber();
        } else {
            std::cout << "[System] Redis 未连接，将独立运行（无外部控制）" << std::endl;
        }

        stream_server_ = std::make_unique<StreamServer>(&buffer_);
        audio_player_ = std::make_unique<AudioPlayer>(
            &buffer_, &playlist_, &current_track_, &playlist_mutex_,
            &playlist_metadata_, redis_connected_ ? &redis_ : nullptr);
        web_server_ = std::make_unique<WebServer>(stream_server_.get());

        bool success = true;
        success &= stream_server_->start();
        success &= audio_player_->start();
        success &= web_server_->start();

        if (success) {
            running_ = true;
            std::cout << "\n"
                "╔══════════════════════════════════════════╗\n"
                "║          引擎启动成功！                 ║\n"
                "║                                          ║\n"
                "║  音频流: http://localhost:2240/stream    ║\n"
                "║  健康检查: http://localhost:2240/health  ║\n";
            if (redis_.is_running()) {
                std::cout << "║  Redis:  已连接 (127.0.0.1:6379)        ║\n";
            }
            std::cout
                << "║                                          ║\n"
                "║  按 Ctrl+C 停止引擎                    ║\n"
                "╚══════════════════════════════════════════╝\n" << std::endl;
        }

        return success;
    }

    void stop() {
        bool expected = false;
        if (!stopped_.compare_exchange_strong(expected, true)) return;
        running_ = false;
        std::cout << "[System] 正在停止引擎..." << std::endl;
        if (web_server_) web_server_->stop();
        if (audio_player_) audio_player_->stop();
        if (stream_server_) stream_server_->stop();
        redis_.stop();
        std::cout << "[System] 引擎已停止" << std::endl;
    }

    void wait_for_shutdown() {
        while (running_) {
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
        }
    }

    void request_stop() { running_ = false; }

private:
    void init_playlist() {
        std::lock_guard<std::mutex> lock(playlist_mutex_);
        fs::create_directories("./media");

        // 读取上次保存的播放顺序
        std::vector<std::string> saved_order;
        {
            std::ifstream f("playlist_order.json");
            if (f.is_open()) {
                std::stringstream ss;
                ss << f.rdbuf();
                auto j = crow::json::load(ss.str());
                if (j && j.t() == crow::json::type::List) {
                    for (size_t i = 0; i < j.size(); ++i)
                        saved_order.push_back(std::string(j[i].s()));
                }
            }
        }

        // 扫描 media/ 目录
        std::set<std::string> available;
        try {
            for (const auto& entry : fs::directory_iterator("./media")) {
                if (!entry.is_regular_file()) continue;
                std::string fn = entry.path().filename().string();
                std::string ext = fs::path(fn).extension();
                std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
                for (const auto& sf : Config::SUPPORTED_FORMATS) {
                    if (ext == sf) { available.insert(fn); break; }
                }
            }
        } catch (const fs::filesystem_error& e) {
            std::cerr << "[Init] 扫描目录时出错: " << e.what() << std::endl;
        }

        std::vector<std::string> ordered;
        std::set<std::string> seen;
        for (const auto& fn : saved_order) {
            if (available.count(fn)) { ordered.push_back(fn); seen.insert(fn); }
        }
        std::vector<std::string> new_files;
        for (const auto& fn : available) {
            if (!seen.count(fn)) new_files.push_back(fn);
        }
        std::sort(new_files.begin(), new_files.end());
        for (const auto& fn : new_files) ordered.push_back(fn);

        for (const auto& fn : ordered) {
            playlist_.push_back(fn);
            playlist_metadata_.push_back(MetadataManager::extract_metadata("./media/" + fn));
        }

        if (!playlist_.empty()) {
            std::random_device rd;
            std::mt19937 gen(rd());
            std::uniform_int_distribution<> dis(0, playlist_.size() - 1);
            current_track_ = dis(gen);
        }

        std::cout << "[Init] 在 ./media/ 中找到 " << playlist_.size() << " 个音频文件" << std::endl;
    }

    BroadcastBuffer buffer_{Config::BUFFER_CAPACITY};
    std::vector<std::string> playlist_;
    std::vector<TrackMetadata> playlist_metadata_;
    std::atomic<size_t> current_track_{0};
    mutable std::mutex playlist_mutex_;

    RedisBridge redis_;
    bool redis_connected_{false};
    std::unique_ptr<StreamServer> stream_server_;
    std::unique_ptr<AudioPlayer> audio_player_;
    std::unique_ptr<WebServer> web_server_;

    std::atomic<bool> running_{false};
    std::atomic<bool> stopped_{false};
};

// =============================================================================
// 全局信号处理器
// =============================================================================
RadioServer* g_server_instance = nullptr;

void signal_handler(int) {
    if (g_server_instance) g_server_instance->request_stop();
}

// =============================================================================
// 主函数
// =============================================================================
int main() {
    std::signal(SIGINT, signal_handler);
    std::signal(SIGTERM, signal_handler);
    std::signal(SIGPIPE, SIG_IGN);

    try {
        RadioServer server;
        g_server_instance = &server;

        if (!server.start()) {
            std::cerr << "[System] 引擎启动失败" << std::endl;
            return 1;
        }

        server.wait_for_shutdown();
        std::cout << "\n[System] 收到终止信号，正在关闭引擎..." << std::endl;
        server.stop();
        g_server_instance = nullptr;

    } catch (const std::exception& e) {
        std::cerr << "[System] 致命错误: " << e.what() << std::endl;
        return 1;
    }

    std::cout << "[System] 引擎已退出" << std::endl;
    return 0;
}
