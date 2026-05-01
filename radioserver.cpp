#include <crow_all.h>
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
#include <fstream>
#include <sstream>
#include "sessionmanager.hpp"
#include "metadata.hpp"
#include "embedded_templates.hpp"
namespace fs = std::filesystem;
// =============================================================================
// 配置常量
// =============================================================================
namespace Config {
    constexpr int WEB_PORT = 2240;
    constexpr size_t BUFFER_CAPACITY = 512 * 1024;  // 512KB - for better streaming
    constexpr size_t AUDIO_CHUNK_SIZE = 16384;      // 16KB - improved chunk size
    constexpr int EPOLL_TIMEOUT_MS = 100;
    constexpr int POLL_TIMEOUT_MS = 200;
    constexpr int MAX_EVENTS = 1024;
    constexpr int MAX_CONNECTIONS = 1024;
    
    // 支持的音频格式
    const std::vector<std::string> SUPPORTED_FORMATS = {
        ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"
    };
    
    // 最大上传文件大小 (50MB)
    constexpr size_t MAX_UPLOAD_SIZE = 50 * 1024 * 1024;
}

// =============================================================================
// 线程安全的环形缓冲区
// =============================================================================
class BroadcastBuffer {
public:
    explicit BroadcastBuffer(size_t capacity = Config::BUFFER_CAPACITY) 
        : capacity_(capacity), mask_(capacity - 1), buffer_(capacity) {
        if (capacity == 0 || (capacity & (capacity - 1)) != 0) {
            throw std::runtime_error("Capacity must be power of two");
        }
    }

    // 生产者写入
void push(const char* data, size_t len) {
    if (len == 0 || len > capacity_) return;
    
    std::lock_guard<std::mutex> lock(write_mutex_);
    
    // 检查是否有足够空间
    size_t current_rp = consume_pos_.load(std::memory_order_relaxed);
    size_t current_wp = write_pos_.load(std::memory_order_relaxed);
    size_t used = (current_wp >= current_rp) ? 
                 (current_wp - current_rp) : 
                 (capacity_ - current_rp + current_wp);
    size_t free = capacity_ - used;
    
    if (len > free) {
        // 空间不足，丢弃最旧的数据
        consume_pos_.store((current_rp + (len - free)) & mask_, 
                           std::memory_order_relaxed);
    }
    
    size_t new_wp = (current_wp + len) & mask_;
    
    // 环形写入
    size_t first_seg = std::min(len, capacity_ - (current_wp & mask_));
    std::memcpy(&buffer_[current_wp & mask_], data, first_seg);
    if (first_seg < len) {
        std::memcpy(&buffer_[0], data + first_seg, len - first_seg);
    }
    
    write_pos_.store(new_wp, std::memory_order_release);
    data_cv_.notify_all();
}

    // 消费者读取
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

    // 等待数据到达
    bool wait_for_data(size_t& consume_pos, int timeout_ms = 100) {
        std::unique_lock<std::mutex> lock(cv_mutex_);
        size_t wp = write_pos_.load(std::memory_order_acquire);

        if (consume_pos != wp) return true;

        return data_cv_.wait_for(lock, std::chrono::milliseconds(timeout_ms),
            [this, &consume_pos]() {
                return consume_pos != write_pos_.load(std::memory_order_acquire);
            });
    }

    // 读取当前生产位置（广播线程用来判断是否有新数据）
    size_t current_write_pos() const {
        return write_pos_.load(std::memory_order_acquire);
    }

    // 停止时唤醒所有等待者
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
// 客户端连接管理
// =============================================================================
class ClientConnection {
public:
    ClientConnection(int fd, BroadcastBuffer* buffer)
        : fd_(fd), buffer_(buffer), consume_pos_(0), shutdown_(false) {
        // 设置非阻塞
        int flags = fcntl(fd_, F_GETFL, 0);
        if (flags >= 0) fcntl(fd_, F_SETFL, flags | O_NONBLOCK);
        
        // 禁用Nagle算法
        int yes = 1;
        setsockopt(fd_, IPPROTO_TCP, TCP_NODELAY, &yes, sizeof(yes));
        
        // 设置发送超时
        struct timeval tv{1, 0};  // 1秒
        setsockopt(fd_, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
    }

    ~ClientConnection() {
        close_socket();
    }

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

    // 在 ClientConnection 类中修改：
bool send_header() {
    if (shutdown_ || header_sent_) return true;
    
    // 使用标准的 HTTP 1.1 响应头，防止 Chrome 下载
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
        
        char audio[Config::AUDIO_CHUNK_SIZE];
        size_t bytes = buffer_->read(consume_pos_, audio, sizeof(audio));
        
        if (bytes == 0) return true;  // 没有数据，但不是错误
        
        ssize_t sent = ::send(fd_, audio, bytes, MSG_NOSIGNAL | MSG_DONTWAIT);
        if (sent < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                return true;  // 缓冲区满，稍后再试
            }
            return false;  // 真正的错误
        }
        
        return true;
    }

private:
    int fd_;
    BroadcastBuffer* buffer_;
    size_t consume_pos_;
    std::atomic<bool> shutdown_{false};
    bool header_sent_{false};
};

// =============================================================================
// 流媒体广播器（通过 Crow 的 /stream 路由接受客户端）
// =============================================================================
class StreamServer {
public:
    StreamServer(BroadcastBuffer* buffer)
        : buffer_(buffer), running_(false), epoll_fd_(-1),
          broadcast_watch_pos_(0) {}

    ~StreamServer() {
        stop();
    }

    bool start() {
        if (running_) return false;

        epoll_fd_ = epoll_create1(0);
        if (epoll_fd_ < 0) {
            perror("epoll_create1");
            return false;
        }

        running_ = true;
        broadcast_watch_pos_ = buffer_->current_write_pos();
        thread_ = std::thread(&StreamServer::worker_loop, this);
        broadcast_thread_ = std::thread(&StreamServer::broadcast_loop, this);

        std::cout << "[Stream] Broadcaster started (serving at /stream)" << std::endl;
        return true;
    }

    void stop() {
        if (!running_.exchange(false)) return;

        buffer_->wakeup_all();

        if (thread_.joinable()) thread_.join();
        if (broadcast_thread_.joinable()) broadcast_thread_.join();

        {
            std::lock_guard<std::mutex> lock(clients_mutex_);
            for (auto& client : clients_) {
                client.second->close_socket();
            }
            clients_.clear();
        }

        if (epoll_fd_ >= 0) {
            close(epoll_fd_);
            epoll_fd_ = -1;
        }

        std::cout << "[Stream] Broadcaster stopped" << std::endl;
    }

    // 由 Crow 的 /stream 路由调用；fd 必须是 dup 后的独立文件描述符。
    void add_client(int fd) {
        if (!running_) { close(fd); return; }

        {
            std::lock_guard<std::mutex> lock(clients_mutex_);
            if (clients_.size() >= Config::MAX_CONNECTIONS) {
                close(fd);
                std::cout << "[Stream] Connection refused: limit reached" << std::endl;
                return;
            }
        }

        auto client = std::make_shared<ClientConnection>(fd, buffer_);

        epoll_event ev{};
        ev.events = EPOLLRDHUP | EPOLLERR | EPOLLHUP;
        ev.data.fd = fd;
        if (epoll_ctl(epoll_fd_, EPOLL_CTL_ADD, fd, &ev) < 0) {
            perror("epoll_ctl add_client");
            close(fd);
            return;
        }

        size_t total;
        {
            std::lock_guard<std::mutex> lock(clients_mutex_);
            clients_[fd] = client;
            total = clients_.size();
        }

        std::cout << "[Stream] New client fd=" << fd << " (total: " << total << ")" << std::endl;
    }

    size_t client_count() const {
        std::lock_guard<std::mutex> lock(clients_mutex_);
        return clients_.size();
    }

private:
    void worker_loop() {
        epoll_event events[Config::MAX_EVENTS];

        while (running_) {
            int n = epoll_wait(epoll_fd_, events, Config::MAX_EVENTS, Config::EPOLL_TIMEOUT_MS);
            if (n < 0) {
                if (errno == EINTR) continue;
                perror("epoll_wait");
                break;
            }

            for (int i = 0; i < n; ++i) {
                int fd = events[i].data.fd;
                uint32_t events_mask = events[i].events;
                if (events_mask & (EPOLLERR | EPOLLRDHUP | EPOLLHUP)) {
                    remove_client(fd);
                }
            }

            static int cleanup_counter = 0;
            if (++cleanup_counter >= 100) {
                cleanup_counter = 0;
                cleanup_dead_clients();
            }
        }
    }

    void broadcast_loop() {
        while (running_) {
            buffer_->wait_for_data(broadcast_watch_pos_, 200);
            if (!running_) break;

            size_t wp = buffer_->current_write_pos();
            if (wp == broadcast_watch_pos_) continue;
            broadcast_watch_pos_ = wp;

            broadcast_audio();
        }
    }

    void remove_client(int fd) {
        std::lock_guard<std::mutex> lock(clients_mutex_);
        auto it = clients_.find(fd);
        if (it != clients_.end()) {
            epoll_ctl(epoll_fd_, EPOLL_CTL_DEL, fd, nullptr);
            it->second->close_socket();
            clients_.erase(it);
            std::cout << "[Stream] Client disconnected fd=" << fd
                      << " (remaining: " << clients_.size() << ")" << std::endl;
        }
    }

    void broadcast_audio() {
        std::vector<std::pair<int, std::shared_ptr<ClientConnection>>> snapshot;
        {
            std::lock_guard<std::mutex> lock(clients_mutex_);
            snapshot.assign(clients_.begin(), clients_.end());
        }

        std::vector<int> dead;
        for (auto& [fd, client] : snapshot) {
            if (client->is_shutdown()) {
                dead.push_back(fd);
                continue;
            }
            if (!client->send_header() || !client->send_audio()) {
                dead.push_back(fd);
            }
        }

        for (int fd : dead) remove_client(fd);
    }

    void cleanup_dead_clients() {
        std::vector<int> dead_clients;
        {
            std::lock_guard<std::mutex> lock(clients_mutex_);
            for (const auto& [fd, client] : clients_) {
                if (client->is_shutdown()) dead_clients.push_back(fd);
            }
        }
        for (int fd : dead_clients) remove_client(fd);
    }

    BroadcastBuffer* buffer_;
    std::atomic<bool> running_{false};
    std::thread thread_;
    std::thread broadcast_thread_;

    int epoll_fd_;
    size_t broadcast_watch_pos_;

    mutable std::mutex clients_mutex_;
    std::unordered_map<int, std::shared_ptr<ClientConnection>> clients_;
};

// =============================================================================
// 音频播放管理器
// =============================================================================
class AudioPlayer {
public:
    AudioPlayer(BroadcastBuffer* buffer, std::vector<std::string>* playlist, 
            std::atomic<size_t>* current_track, std::mutex* playlist_mutex)
    : buffer_(buffer), playlist_(playlist), current_track_(current_track),
      playlist_mutex_(playlist_mutex), running_(false), skip_track_(false) {}

    
    ~AudioPlayer() {
        stop();
    }

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
        // 唤醒可能阻塞在空播放列表等待中的工作线程
        {
            std::lock_guard<std::mutex> lock(idle_mutex_);
            idle_cv_.notify_all();
        }
        if (thread_.joinable()) thread_.join();

        std::cout << "[Audio] Player stopped" << std::endl;
    }

    void skip_current_track() {
        skip_track_ = true;
    }

    void load_file(const std::string& filename) {
        // 简单实现：设置跳过标志，主循环会自动播放新文件
        skip_track_ = true;
    }

private:
    BroadcastBuffer* buffer_;
    std::vector<std::string>* playlist_;
    std::atomic<size_t>* current_track_;
    std::mutex* playlist_mutex_;

    std::atomic<bool> running_{false};
    std::atomic<bool> skip_track_{false};
    std::thread thread_;
    std::mutex idle_mutex_;
    std::condition_variable idle_cv_;

    // 可被 stop() 立刻唤醒的睡眠；空列表/错误恢复等待时用它代替 sleep_for，
    // 以便 SIGTERM 能及时让 stop() 完成，无需 SIGKILL 兜底。
    void interruptible_wait(std::chrono::milliseconds dur) {
        std::unique_lock<std::mutex> lock(idle_mutex_);
        idle_cv_.wait_for(lock, dur, [this]() { return !running_.load(); });
    }

    void worker_loop() {
        while (running_) {
            // 等待播放列表中有音乐
            bool playlist_empty = false;
            {
                std::lock_guard<std::mutex> lock(*playlist_mutex_);
                playlist_empty = playlist_->empty();
            }

            if (playlist_empty) {
                // 播放列表为空时等待较长时间以降低 CPU，但能被 stop() 立即唤醒
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

    try {
        std::string filename;

        {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            playlist_size = playlist_->size();
            if (playlist_size == 0) return;  // 提前返回

            // 安全的模运算
            size_t track_idx = playlist_size > 0 ?
                               current_track_->load() % playlist_size : 0;
            filename = "./media/" + playlist_->at(track_idx);

            std::cout << "[Audio] Playing: " << playlist_->at(track_idx)
                      << " (" << track_idx + 1 << "/" << playlist_->size() << ")" << std::endl;
        }

        // 检查文件是否存在
        if (!fs::exists(filename)) {
            std::cerr << "[Audio] File not found: " << filename << std::endl;
            std::cerr << "[Audio] Current working directory: " << fs::current_path() << std::endl;
            (*current_track_)++;
            // 文件不存在时增加延迟，避免快速循环消耗CPU（可被 stop 唤醒）
            interruptible_wait(std::chrono::seconds(1));
            return;
        }

        std::cout << "[Audio] Processing file: " << filename << std::endl;

        // 不通过 shell 调用 ffmpeg，避免文件名中的特殊字符触发命令注入。
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
            close(fds[0]);
            close(fds[1]);
            (*current_track_)++;
            interruptible_wait(std::chrono::seconds(1));
            return;
        }

        if (child_pid == 0) {
            // 子进程：把管道写端接到 stdout，stderr 留给父进程的日志。
            if (dup2(fds[1], STDOUT_FILENO) < 0) _exit(127);
            // O_CLOEXEC 会在 exec 后自动关闭 fds[0]/fds[1]
            const char* argv[] = {
                "ffmpeg", "-nostdin", "-re", "-loglevel", "error",
                "-i", filename.c_str(),
                "-vn", "-c:a", "libmp3lame", "-b:a", "128k",
                "-ar", "44100", "-ac", "2", "-f", "mp3", "pipe:1",
                nullptr
            };
            execvp("ffmpeg", const_cast<char* const*>(argv));
            _exit(127);
        }

        // 父进程：只读管道
        close(fds[1]);
        pipe_fd = fds[0];

        // 使用poll监控管道
        struct pollfd pfd = {pipe_fd, POLLIN, 0};

        char buffer[Config::AUDIO_CHUNK_SIZE];
        skip_track_ = false;

        bool error_occurred = false;

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
                    } else if (bytes == 0) {
                        break;  // EOF
                    } else if (bytes < 0) {
                        if (errno != EAGAIN && errno != EWOULDBLOCK) {
                            std::cerr << "[Audio] Read error: " << strerror(errno) << std::endl;
                            error_occurred = true;
                        }
                    }
                } else if (pfd.revents & POLLHUP) {
                    break;  // Normal EOF: FFmpeg finished encoding
                } else if (pfd.revents & (POLLERR | POLLNVAL)) {
                    std::cerr << "[Audio] Pipe error: revents=" << pfd.revents << std::endl;
                    error_occurred = true;
                }
            } else if (ret < 0) {
                if (errno == EINTR) continue;
                std::cerr << "[Audio] Poll error: " << strerror(errno) << std::endl;
                error_occurred = true;
            }
            // 超时继续检查条件
        }
        } catch (const std::exception& e) {
            std::cerr << "[Audio] Error playing track: " << e.what() << std::endl;
        }

        // 关闭管道并回收子进程；SIGTERM + 超时后 SIGKILL
        if (pipe_fd >= 0) {
            close(pipe_fd);
            pipe_fd = -1;
        }
        if (child_pid > 0) {
            kill(child_pid, SIGTERM);
            int status = 0;
            int waited = 0;
            const int max_wait_ms = 2000;
            const int tick_ms = 50;
            while (waited < max_wait_ms) {
                pid_t w = waitpid(child_pid, &status, WNOHANG);
                if (w > 0) break;
                if (w < 0 && errno != EINTR) break;
                std::this_thread::sleep_for(std::chrono::milliseconds(tick_ms));
                waited += tick_ms;
            }
            if (waited >= max_wait_ms) {
                kill(child_pid, SIGKILL);
                while (waitpid(child_pid, &status, 0) < 0) {
                    if (errno != EINTR) break;
                }
            }
            child_pid = -1;
        }

        // 检查是否需要切换到下一首（播放完成且用户没有手动切换）
        // 注意：如果用户通过API切换了轨道，skip_track_会被设置为true
        // 这种情况下不应该自动递增current_track_，因为用户已经设置了新的索引
        if (!skip_track_ && running_) {
            // 只有当正常播放完成且没有用户干预时，才自动切换到下一首
            // 用录制时的 playlist_size 取模，避免索引无限累积越界
            current_track_->store((current_track_->load() + 1) % (playlist_size > 0 ? playlist_size : 1));
        }
        // 重置跳过标志，为下一轮播放做准备
        skip_track_ = false;
    }
};

// =============================================================================
// Web服务器管理器
// =============================================================================
// =============================================================================
// Web服务器管理器（更新版，整合认证系统）
// =============================================================================
class WebServer {
public:
    struct Config {
        bool allow_guest_skip = false;  // 是否允许游客切歌
        std::string admin_password = "";
        std::string station_name = "我的音乐电台";
        std::string subtitle = "极简流媒体服务器";
        std::string primary_color = "#764ba2";
        std::string secondary_color = "#667eea";
        std::string bg_color = "#f4f4f9";
        static constexpr int WEB_PORT = 2240;
        static const std::vector<std::string> SUPPORTED_FORMATS;
        static constexpr size_t MAX_UPLOAD_SIZE = 50 * 1024 * 1024;
        
        static Config load_from_settings() {
            Config config;
            std::ifstream conf_file("settings.json");
            if (conf_file.is_open()) {
                std::stringstream ss;
                ss << conf_file.rdbuf();
                auto j = crow::json::load(ss.str());
                if (j) {
                    if (j.has("allow_guest_skip")) 
                        config.allow_guest_skip = j["allow_guest_skip"].b();
                    if (j.has("admin_password")) 
                        config.admin_password = j["admin_password"].s();
                    if (j.has("station_name")) 
                        config.station_name = j["station_name"].s();
                    if (j.has("subtitle")) 
                        config.subtitle = j["subtitle"].s();
                    if (j.has("primary_color")) 
                        config.primary_color = j["primary_color"].s();
                    if (j.has("secondary_color")) 
                        config.secondary_color = j["secondary_color"].s();
                    if (j.has("bg_color")) 
                        config.bg_color = j["bg_color"].s();
                }
            }
            
            // 如果未设置管理员密码，使用默认密码
            if (config.admin_password.empty()) {
                config.admin_password = "admin123";
                std::cout << "[Web] 警告：使用默认管理员密码: admin123" << std::endl;
                std::cout << "[Web] 请在 settings.json 中设置 admin_password" << std::endl;
            }
            
            return config;
        }
    };

    WebServer(std::vector<std::string>* playlist, std::vector<TrackMetadata>* playlist_metadata,
          std::atomic<size_t>* current_track, StreamServer* stream_server,
          AudioPlayer* audio_player, std::mutex* playlist_mutex)
    : config_(Config::load_from_settings()),
      playlist_(playlist), playlist_metadata_(playlist_metadata), current_track_(current_track),
      stream_server_(stream_server), audio_player_(audio_player),
      playlist_mutex_(playlist_mutex), running_(false) {
// 简单认证系统 - 直接使用 SessionManager
        session_manager_ = std::make_unique<SessionManager>();
        
        std::cout << "[Web] 加载配置文件成功" << std::endl;
        std::cout << "[Web] 管理员密码已配置" << std::endl;
        std::cout << "[Web] 允许游客切歌: " << (config_.allow_guest_skip ? "是" : "否") << std::endl;
    }
    
    ~WebServer() {
        stop();
    }
    
    bool start() {
        if (running_) return false;
        
        setup_routes();
        
        running_ = true;
        
        thread_ = std::thread([this]() {
            try {
                std::cout << "[Web] 服务器启动在端口 " << WebServer::Config::WEB_PORT << std::endl;
                app_.signal_clear();
                app_.port(WebServer::Config::WEB_PORT).multithreaded().run();
            } catch (const std::exception& e) {
                std::cerr << "[Web] 错误: " << e.what() << std::endl;
            }
            running_ = false;
        });
        
        return true;
    }
    
    void stop() {
        if (!running_.exchange(false)) return;
        
        app_.stop();
        if (thread_.joinable()) thread_.join();
        
        std::cout << "[Web] 服务器已停止" << std::endl;
    }

private:
    // 辅助函数：替换字符串中所有的指定内容
    static void replace_all(std::string& str, const std::string& from, const std::string& to) {
        if(from.empty()) return;
        size_t start_pos = 0;
        while((start_pos = str.find(from, start_pos)) != std::string::npos) {
            str.replace(start_pos, from.length(), to);
            start_pos += to.length();
        }
    }
    
    // 读取HTML模板并替换变量
    std::string render_template(const std::string& filename,
                       const std::map<std::string, std::string>& context = {},
                       bool is_admin = false) {
        std::string html_content;

        // 优先读磁盘文件（支持运行时覆盖）
        for (const auto& path : {filename, "templates/" + filename}) {
            std::ifstream f(path);
            if (f.is_open()) {
                std::stringstream ss;
                ss << f.rdbuf();
                html_content = ss.str();
                break;
            }
        }

        // 磁盘找不到时回落到编译期内嵌模板
        if (html_content.empty()) {
            auto it = EmbeddedTemplates::templates.find(filename);
            if (it == EmbeddedTemplates::templates.end())
                throw std::runtime_error("无法找到模板: " + filename);
            html_content = it->second;
        }
        
        // 基本配置替换
        replace_all(html_content, "{{STATION_NAME}}", config_.station_name);
        replace_all(html_content, "{{SUBTITLE}}", config_.subtitle);
        replace_all(html_content, "{{PRIMARY_COLOR}}", config_.primary_color);
        replace_all(html_content, "{{SECONDARY_COLOR}}", config_.secondary_color);
        replace_all(html_content, "{{BG_COLOR}}", config_.bg_color);
        
        // 权限相关的替换
        replace_all(html_content, "{{IS_ADMIN}}", is_admin ? "true" : "false");
        
        // 用户定义的上下文变量
        for (const auto& [key, value] : context) {
            replace_all(html_content, "{{" + key + "}}", value);
        }
        
        return html_content;
    }
    
    // 读取静态文件：磁盘优先（支持运行时替换），否则回落到内嵌模板
    static std::string read_static_asset(const std::string& filename) {
        for (const auto& path : {filename, "templates/" + filename}) {
            std::ifstream f(path, std::ios::binary);
            if (f.is_open()) {
                std::stringstream ss;
                ss << f.rdbuf();
                return ss.str();
            }
        }
        auto it = EmbeddedTemplates::templates.find(filename);
        if (it != EmbeddedTemplates::templates.end()) return it->second;
        return {};
    }

    void setup_routes() {
        // PWA manifest：需要替换 {{STATION_NAME}} 等占位符
        CROW_ROUTE(app_, "/manifest.json")([this]() {
            try {
                crow::response res(render_template("manifest.json", {}, false));
                res.set_header("Content-Type", "application/manifest+json; charset=utf-8");
                res.set_header("Cache-Control", "public, max-age=3600");
                return res;
            } catch (const std::exception& e) {
                return crow::response(404, std::string("manifest 不可用: ") + e.what());
            }
        });

        // PWA 图标：内联 SVG（紫色音符图标）
        CROW_ROUTE(app_, "/icon.svg")([]() {
            std::string svg = R"svg(
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192">
  <rect width="192" height="192" rx="32" fill="#764ba2"/>
  <text x="96" y="128" text-anchor="middle" font-size="96" fill="white">🎵</text>
</svg>)svg";
            crow::response res(svg);
            res.set_header("Content-Type", "image/svg+xml");
            res.set_header("Cache-Control", "public, max-age=86400");
            return res;
        });

        // Service Worker：原样返回 JS，并允许其作用于根路径
        CROW_ROUTE(app_, "/sw.js")([]() {
            std::string body = read_static_asset("sw.js");
            if (body.empty()) return crow::response(404, "sw.js 不可用");
            crow::response res(body);
            res.set_header("Content-Type", "application/javascript; charset=utf-8");
            res.set_header("Service-Worker-Allowed", "/");
            res.set_header("Cache-Control", "no-cache");
            return res;
        });

        // 音频流端点：接管底层 socket 并注册到广播器
        CROW_ROUTE(app_, "/stream")([this](const crow::request& /*req*/, crow::response& res) {
            if (!res.get_socket_fd_helper_) {
                res.code = 500;
                res.end("stream unavailable");
                return;
            }
            int crow_fd = res.get_socket_fd_helper_();
            int fd = ::dup(crow_fd);
            if (fd < 0) {
                res.code = 500;
                res.end("dup failed");
                return;
            }
            res.take_over();
            stream_server_->add_client(fd);
        });

        // 主页 - 根据是否登录显示不同界面
        CROW_ROUTE(app_, "/")([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);

            try {
                if (is_admin) {
                    // 管理员显示管理面板
                    auto admin_context = std::map<std::string, std::string>{
                        {"CLIENT_COUNT", std::to_string(stream_server_->client_count())}
                    };

                    // 获取播放列表信息
                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    admin_context["TRACK_COUNT"] = std::to_string(playlist_->size());
                    admin_context["CURRENT_TRACK"] = std::to_string(current_track_->load() + 1);

                    return crow::response(render_template("panel.html", admin_context, true));
                } else {
                    // 普通用户显示收听界面
                    return crow::response(render_template("index.html", {
                        {"ALLOW_GUEST_SKIP", config_.allow_guest_skip ? "true" : "false"}
                    }, false));
                }
            } catch (const std::exception& e) {
                // 如果模板不存在，返回错误
                return crow::response(500, std::string("模板错误: ") + e.what());
            }
        });
        
        // 管理员登录页面
        CROW_ROUTE(app_, "/admin")([this](const crow::request& req) {
            // 检查用户是否已经登录
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);

            if (is_admin) {
                // 如果已经登录，重定向到管理面板
                crow::response res(302);
                res.set_header("Location", "/");
                return res;
            }
            
            try {
                return crow::response(render_template("login.html", {}, false));
            } catch (const std::exception& e) {
                // 如果模板不存在，返回简单登录页面
                std::string simple_login = R"html(
<!DOCTYPE html>
<html>
<head><title>管理员登录</title><style>body{font-family:Arial;text-align:center;padding:50px}</style></head>
<body>
    <h1>管理员登录</h1>
    <div style="max-width:300px;margin:20px auto;">
        <input type="password" id="password" placeholder="密码" style="padding:10px;width:100%;margin:10px 0;">
        <button onclick="login()" style="padding:10px 20px;background:#764ba2;color:white;border:none;cursor:pointer;width:100%;">登录</button>
    </div>
    <script>
    async function login() {
        const password = document.getElementById('password').value;
        const response = await fetch('/admin/login', {
            method: 'POST',
            headers: {'Content-Type': 'application/json'},
            body: JSON.stringify({password: password})
        });
        if (response.ok) window.location.href = '/';
        else alert('密码错误');
    }
    </script>
</body>
</html>
                )html";
                replace_all(simple_login, "{{STATION_NAME}}", config_.station_name);
                return crow::response(simple_login);
            }
        });
        
        // 登录API
        CROW_ROUTE(app_, "/admin/login").methods("POST"_method)([this](const crow::request& req) {
            try {
                auto j = crow::json::load(req.body);
                if (!j || !j.has("password")) {
                    return crow::response(400, "缺少密码参数");
                }
                
                std::string password = j["password"].s();
                if (SessionManager::constant_time_str_eq(config_.admin_password, password)) {
                    auto session = session_manager_->create_admin_session();
                    crow::response res(200);
                    res.set_header("Set-Cookie", 
                        "session_id=" + session->session_id + 
                        "; Path=/; HttpOnly; Max-Age=86400; SameSite=Lax");
                    res.write("登录成功");
                    return res;
                }
                return crow::response(401, "密码错误");
            } catch (const std::exception& e) {
                return crow::response(400, std::string("请求格式错误: ") + e.what());
            }
        });
        
        // 登出API
        CROW_ROUTE(app_, "/admin/logout").methods("POST"_method)([this](const crow::request& req) {
            // 从Cookie中获取session_id并销毁会话
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!session_id.empty()) {
                session_manager_->destroy_session(session_id);
            }

            crow::response res(200);
            res.set_header("Set-Cookie",
                "session_id=; Path=/; HttpOnly; Max-Age=0; SameSite=Lax");
            res.write("登出成功");
            return res;
        });
        
        // 公开API：播放列表信息
        CROW_ROUTE(app_, "/api/playlist")([this]() {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            crow::json::wvalue result;

            // 返回文件名播放列表（向后兼容）
            result["playlist"] = *playlist_;
            result["current"] = playlist_->empty() ? 0 : (int)(current_track_->load() % playlist_->size());

            // 返回元数据播放列表
            std::vector<crow::json::wvalue> metadata_list;
            for (size_t i = 0; i < playlist_metadata_->size(); ++i) {
                const TrackMetadata& metadata = playlist_metadata_->at(i);
                crow::json::wvalue item;
                item["filename"] = metadata.filename;
                item["title"] = metadata.get_display_name();
                item["artist"] = metadata.artist;
                item["album"] = metadata.album;
                item["duration"] = metadata.duration;
                item["cover_url"] = "/api/cover/" + std::to_string(i);
                item["metadata_url"] = "/api/metadata/" + std::to_string(i);
                metadata_list.push_back(item);
            }
            result["metadata"] = std::move(metadata_list);

            return crow::response(result);
        });
        
        // 公开API：统计信息
        CROW_ROUTE(app_, "/api/stats")([this]() {
            crow::json::wvalue result;
            result["clients"] = (int)stream_server_->client_count();
            return crow::response(result);
        });
        
        // 需要权限的API：上传文件（仅管理员）
        CROW_ROUTE(app_, "/upload").methods("POST"_method)([this](const crow::request& req) {
            // 检查用户是否为管理员
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);
            if (!is_admin) {
                return crow::response(403, "需要管理员权限才能上传文件");
            }

            return handle_upload(req);
        });
        
        // 需要权限的API：下一首（如果允许游客切歌，则游客也可以使用）
        CROW_ROUTE(app_, "/api/next").methods("POST"_method)([this](const crow::request& req) {
            // 检查用户是否已登录或允许游客切歌
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);
            if (!is_admin && !config_.allow_guest_skip) {
                return crow::response(403, "需要登录才能执行此操作");
            }
            
            // 权限检查通过，执行切歌
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            if (playlist_->empty()) return crow::response{400, "播放列表为空"};
            
            size_t new_index = (current_track_->load() + 1) % playlist_->size();
            current_track_->store(new_index);
            audio_player_->skip_current_track();
            
            return crow::response{200, "跳到下一首"};
        });
        
        // 需要权限的API：上一首（如果允许游客切歌，则游客也可以使用）
        CROW_ROUTE(app_, "/api/prev").methods("POST"_method)([this](const crow::request& req) {
            // 检查用户是否已登录或允许游客切歌
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);
            if (!is_admin && !config_.allow_guest_skip) {
                return crow::response(403, "需要登录才能执行此操作");
            }
            
            // 权限检查通过，执行切歌
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            if (playlist_->empty()) return crow::response{400, "播放列表为空"};
            
            size_t size = playlist_->size();
            size_t new_index = (current_track_->load() + size - 1) % size;
            current_track_->store(new_index);
            audio_player_->skip_current_track();
            
            return crow::response{200, "跳到上一首"};
        });
        
        // 需要权限的API：播放指定歌曲（如果允许游客切歌，则游客也可以使用）
        CROW_ROUTE(app_, "/api/play/<int>").methods("POST"_method)([this](const crow::request& req, int idx) {
            // 检查用户是否已登录或允许游客切歌
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);
            if (!is_admin && !config_.allow_guest_skip) {
                return crow::response(403, "需要登录才能执行此操作");
            }
            
            // 权限检查通过，执行切歌
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            if (playlist_->empty()) return crow::response{400, "播放列表为空"};
            
            size_t index = static_cast<size_t>(idx);
            if (index >= playlist_->size()) return crow::response{400, "索引超出范围"};
            
            current_track_->store(index);
            audio_player_->skip_current_track();
            
            return crow::response{200, "播放歌曲: " + std::to_string(index)};
        });
        
        // 需要权限的API：删除歌曲（仅管理员）
        CROW_ROUTE(app_, "/api/delete/<int>").methods("POST"_method)([this](const crow::request& req, int idx) {
            // 检查用户是否为管理员
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            bool is_admin = check_admin_auth(session_id);
            if (!is_admin) {
                return crow::response(403, "需要管理员权限才能执行此操作");
            }
            
            std::lock_guard<std::mutex> lock(*playlist_mutex_);
            if (playlist_->empty()) return crow::response{400, "播放列表为空"};
            
            size_t index = static_cast<size_t>(idx);
            if (index >= playlist_->size()) return crow::response{400, "索引超出范围"};
            
            size_t current = current_track_->load();

            // 同步删除两个并行向量
            playlist_->erase(playlist_->begin() + index);
            if (index < playlist_metadata_->size()) {
                playlist_metadata_->erase(playlist_metadata_->begin() + index);
            }

            // 调整当前播放索引
            if (playlist_->empty()) {
                current_track_->store(0);
            } else if (index < current) {
                // 删除的曲目在当前曲目之前，索引需要减1
                current_track_->store(current - 1);
            } else if (index == current) {
                // 删除了正在播放的曲目，当前索引不变（自动指向下一首）
                audio_player_->skip_current_track();
            } else if (current >= playlist_->size()) {
                // current > index 且超出范围，回绕到开头
                current_track_->store(0);
                audio_player_->skip_current_track();
            }
            // index > current: 删除当前曲目之后，索引无需调整

            save_playlist_order();
            return crow::response{200, "删除成功"};
        });

        // 元数据API：获取指定歌曲详细元数据
        CROW_ROUTE(app_, "/api/metadata/<int>")([this](int idx) {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);

            size_t index = static_cast<size_t>(idx);
            if (index >= playlist_metadata_->size()) {
                return crow::response(404, "索引超出范围");
            }

            const TrackMetadata& metadata = playlist_metadata_->at(index);
            crow::json::wvalue result;
            result["filename"] = metadata.filename;
            result["title"] = metadata.title;
            result["artist"] = metadata.artist;
            result["album"] = metadata.album;
            result["genre"] = metadata.genre;
            result["year"] = metadata.year;
            result["track_number"] = metadata.track_number;
            result["duration"] = metadata.duration;
            result["has_cover"] = !metadata.cover_art.empty();
            result["has_lyrics"] = !metadata.lyrics.empty();

            return crow::response(result);
        });

        // 专辑封面API：获取专辑封面图片
        CROW_ROUTE(app_, "/api/cover/<int>")([this](int idx) {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);

            size_t index = static_cast<size_t>(idx);
            if (index >= playlist_metadata_->size()) {
                return crow::response(404, "索引超出范围");
            }

            const TrackMetadata& metadata = playlist_metadata_->at(index);
            if (metadata.cover_art.empty()) {
                return crow::response(404, "无专辑封面");
            }

            crow::response res;
            res.set_header("Content-Type", "image/jpeg");
            res.set_header("Cache-Control", "public, max-age=3600");
            res.write(std::string(metadata.cover_art.begin(), metadata.cover_art.end()));

            return res;
        });

        // 歌词API：获取歌词文本
        CROW_ROUTE(app_, "/api/lyrics/<int>")([this](int idx) {
            std::lock_guard<std::mutex> lock(*playlist_mutex_);

            size_t index = static_cast<size_t>(idx);
            if (index >= playlist_metadata_->size()) {
                return crow::response(404, "索引超出范围");
            }

            const TrackMetadata& metadata = playlist_metadata_->at(index);
            if (metadata.lyrics.empty()) {
                return crow::response(404, "无歌词");
            }

            return crow::response(metadata.lyrics);
        });

        // 管理员API：启动批量下载任务
        CROW_ROUTE(app_, "/admin/download").methods("POST"_method)([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            if (dl_running_.load())
                return crow::response(409, "下载任务正在运行中，请等待完成");

            auto body = crow::json::load(req.body);
            if (!body || !body.has("playlist"))
                return crow::response(400, "缺少 playlist 字段");

            std::string playlist_text = std::string(body["playlist"].s());
            if (playlist_text.empty())
                return crow::response(400, "歌单不能为空");

            static const std::set<std::string> VALID_QUALITY = {"standard","high","exhigh","lossless"};
            static const std::set<std::string> VALID_FORMAT  = {"mp3","flac","m4a","opus"};
            std::string quality = (body.has("quality") && VALID_QUALITY.count(std::string(body["quality"].s())))
                                  ? std::string(body["quality"].s()) : "exhigh";
            std::string dl_fmt  = (body.has("format")  && VALID_FORMAT.count(std::string(body["format"].s())))
                                  ? std::string(body["format"].s())  : "mp3";

            char tmp_path[] = "/tmp/rakuraku_XXXXXX";
            int fd = mkstemp(tmp_path);
            if (fd == -1)
                return crow::response(500, "无法创建临时文件");
            write(fd, playlist_text.data(), playlist_text.size());
            close(fd);

            dl_running_ = true;
            {
                std::lock_guard<std::mutex> lock(dl_mutex_);
                dl_log_ = "[下载任务已启动]\n";
            }

            std::thread([this, tmp_str = std::string(tmp_path), quality, dl_fmt]() {
                std::string cmd = "python3 -u ./music_dl.py --settings ./settings.json"
                                + std::string(" -o ./media/ -q ") + quality
                                + " -f " + dl_fmt + " " + tmp_str + " 2>&1";
                FILE* pipe = popen(cmd.c_str(), "r");
                if (!pipe) {
                    std::lock_guard<std::mutex> lock(dl_mutex_);
                    dl_log_ += "[错误：无法启动 music_dl.py，请确认 Python 依赖已安装]\n";
                    dl_running_ = false;
                    unlink(tmp_str.c_str());
                    return;
                }
                char buf[512];
                while (fgets(buf, sizeof(buf), pipe)) {
                    std::lock_guard<std::mutex> lock(dl_mutex_);
                    dl_log_ += buf;
                    if (dl_log_.size() > 20000)
                        dl_log_ = dl_log_.substr(dl_log_.size() - 20000);
                }
                pclose(pipe);
                unlink(tmp_str.c_str());
                {
                    std::lock_guard<std::mutex> lock(dl_mutex_);
                    dl_log_ += "\n[任务结束]\n";
                }
                dl_running_ = false;

                // 下载完成后，将新文件加入播放列表
                static const std::vector<std::string> AUDIO_EXTS = {".mp3",".wav",".flac",".ogg",".m4a",".aac"};
                std::lock_guard<std::mutex> plock(*playlist_mutex_);
                std::set<std::string> existing(playlist_->begin(), playlist_->end());
                for (const auto& entry : fs::directory_iterator("./media/")) {
                    if (!entry.is_regular_file()) continue;
                    std::string ext = entry.path().extension().string();
                    std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
                    if (std::find(AUDIO_EXTS.begin(), AUDIO_EXTS.end(), ext) == AUDIO_EXTS.end()) continue;
                    std::string fname = entry.path().filename().string();
                    if (!existing.count(fname)) {
                        playlist_->push_back(fname);
                        playlist_metadata_->push_back(MetadataManager::extract_metadata(entry.path().string()));
                    }
                }
                save_playlist_order();
            }).detach();

            crow::json::wvalue result;
            result["status"] = "started";
            return crow::response(result);
        });

        // 管理员API：查询批量下载任务状态
        CROW_ROUTE(app_, "/admin/download/status")([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            crow::json::wvalue result;
            result["running"] = dl_running_.load();
            std::lock_guard<std::mutex> lock(dl_mutex_);
            result["log"] = dl_log_;
            return crow::response(result);
        });

        // 管理员API：查询网易云账号配置状态（不返回密码/Cookie原文）
        CROW_ROUTE(app_, "/admin/settings/ncm")([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            crow::json::wvalue result;
            result["configured"] = false;
            result["method"] = "none";

            std::ifstream f("settings.json");
            if (f.is_open()) {
                std::stringstream ss;
                ss << f.rdbuf();
                auto j = crow::json::load(ss.str());
                if (j) {
                    bool has_cookie = j.has("ncm_cookie") && !std::string(j["ncm_cookie"].s()).empty();
                    bool has_phone  = j.has("ncm_phone")  && !std::string(j["ncm_phone"].s()).empty();
                    if (has_cookie) {
                        result["configured"] = true;
                        result["method"] = "cookie";
                    } else if (has_phone) {
                        result["configured"] = true;
                        result["method"] = "phone";
                        std::string phone = std::string(j["ncm_phone"].s());
                        result["phone_hint"] = phone.substr(0, 3) + "****" + phone.substr(phone.size() > 7 ? phone.size() - 4 : 3);
                    }
                }
            }
            return crow::response(result);
        });

        // 管理员API：保存网易云账号凭据
        CROW_ROUTE(app_, "/admin/settings/ncm").methods("POST"_method)([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            auto body = crow::json::load(req.body);
            if (!body) return crow::response(400, "无效的请求");

            std::string phone  = body.has("phone")    ? std::string(body["phone"].s())    : "";
            std::string pass   = body.has("password") ? std::string(body["password"].s()) : "";
            std::string cookie = body.has("cookie")   ? std::string(body["cookie"].s())   : "";

            if (!save_ncm_settings(phone, pass, cookie))
                return crow::response(500, "无法写入 settings.json");

            return crow::response(200, "保存成功");
        });

        // 管理员API：测试网易云账号登录
        CROW_ROUTE(app_, "/admin/settings/ncm/test").methods("POST"_method)([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            FILE* pipe = popen("python3 -u ./music_dl.py --verify-login --settings ./settings.json 2>&1", "r");
            if (!pipe) return crow::response(500, "无法启动测试进程");

            std::string output;
            char buf[256];
            while (fgets(buf, sizeof(buf), pipe)) output += buf;
            int ret = pclose(pipe);

            crow::json::wvalue result;
            result["success"] = (ret == 0);
            result["output"]  = output;
            return crow::response(result);
        });

        // 保存settings.json（通用）
        CROW_ROUTE(app_, "/admin/settings/save").methods("POST"_method)([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            try {
                auto j = crow::json::load(req.body);
                if (!j) return crow::response(400, "无效的JSON数据");

                std::string raw = "{}";
                {
                    std::ifstream f("settings.json");
                    if (f.is_open()) { std::stringstream ss; ss << f.rdbuf(); raw = ss.str(); }
                }
                auto old_j = crow::json::load(raw);
                crow::json::wvalue out;

                // 起点：完整复制现有 settings.json，仅更新请求中显式提供的字段
                if (old_j) {
                    for (const auto& pair : old_j.keys()) {
                        std::string key(pair);
                        switch (old_j[key].t()) {
                            case crow::json::type::String:
                                out[key] = std::string(old_j[key].s()); break;
                            case crow::json::type::True:
                            case crow::json::type::False:
                                out[key] = old_j[key].b(); break;
                            case crow::json::type::Number:
                                out[key] = old_j[key].d(); break;
                            default:
                                break;
                        }
                    }
                }

                // 仅覆盖请求中显式提供的字段
                if (j.has("station_name"))
                    out["station_name"] = std::string(j["station_name"].s());

                if (j.has("subtitle"))
                    out["subtitle"] = std::string(j["subtitle"].s());

                if (j.has("primary_color"))
                    out["primary_color"] = std::string(j["primary_color"].s());

                if (j.has("secondary_color"))
                    out["secondary_color"] = std::string(j["secondary_color"].s());

                if (j.has("bg_color"))
                    out["bg_color"] = std::string(j["bg_color"].s());

                if (j.has("allow_guest_skip"))
                    out["allow_guest_skip"] = j["allow_guest_skip"].b();

                if (j.has("ncm_phone"))
                    out["ncm_phone"] = std::string(j["ncm_phone"].s());

                if (j.has("ncm_password"))
                    out["ncm_password"] = std::string(j["ncm_password"].s());

                if (j.has("ncm_cookie"))
                    out["ncm_cookie"] = std::string(j["ncm_cookie"].s());

                // 密码字段非空才更新（避免空字符串误清空已有密码）
                if (j.has("admin_password")) {
                    std::string new_password = std::string(j["admin_password"].s());
                    if (!new_password.empty()) {
                        out["admin_password"] = new_password;
                    }
                }

                // 保存到文件，用独立作用域确保 ofstream 析构（flush+close）后再更新内存
                {
                    std::ofstream f("settings.json");
                    if (!f) return crow::response(500, "无法保存设置");
                    f << out.dump(4);
                } // f 在此处关闭并 flush

                // 直接用已验证的请求字段更新内存配置，避免重读文件时缓冲区未落盘的问题
                if (j.has("station_name"))
                    config_.station_name = std::string(j["station_name"].s());
                if (j.has("subtitle"))
                    config_.subtitle = std::string(j["subtitle"].s());
                if (j.has("primary_color"))
                    config_.primary_color = std::string(j["primary_color"].s());
                if (j.has("secondary_color"))
                    config_.secondary_color = std::string(j["secondary_color"].s());
                if (j.has("bg_color"))
                    config_.bg_color = std::string(j["bg_color"].s());
                if (j.has("allow_guest_skip"))
                    config_.allow_guest_skip = j["allow_guest_skip"].b();
                if (j.has("admin_password")) {
                    std::string new_password = std::string(j["admin_password"].s());
                    if (!new_password.empty())
                        config_.admin_password = new_password;
                }

                return crow::response(200, "设置已保存");
            } catch (const std::exception& e) {
                return crow::response(500, std::string("保存失败: ") + e.what());
            }
        });

        // 获取所有settings.json设置（敏感信息过滤）
        CROW_ROUTE(app_, "/admin/settings/get")([this](const crow::request& req) {
            std::string session_id = get_session_id_from_cookies(req.get_header_value("Cookie"));
            if (!check_admin_auth(session_id))
                return crow::response(403, "需要管理员权限");

            std::ifstream f("settings.json");
            if (!f.is_open()) return crow::response(404, "未找到设置文件");

            std::stringstream ss;
            ss << f.rdbuf();
            auto j = crow::json::load(ss.str());
            if (!j) return crow::response(500, "设置文件格式错误");

            crow::json::wvalue result;

            // 复制所有字段，但过滤密码信息；按实际类型处理（避免对 bool 调 .s() 抛异常）
            for (const auto& pair : j.keys()) {
                std::string key(pair);
                if (key == "admin_password" || key == "ncm_password" || key == "ncm_cookie") {
                    result[key] = "********";
                    continue;
                }
                switch (j[key].t()) {
                    case crow::json::type::String:
                        result[key] = std::string(j[key].s()); break;
                    case crow::json::type::True:
                    case crow::json::type::False:
                        result[key] = j[key].b(); break;
                    case crow::json::type::Number:
                        result[key] = j[key].d(); break;
                    default:
                        break;
                }
            }

            // 添加运行时配置
            result["allow_guest_skip_runtime"] = config_.allow_guest_skip;
            result["station_name_runtime"] = config_.station_name;
            result["subtitle_runtime"] = config_.subtitle;

            // PWA配置
            result["pwa_available"] = true;
            result["pwa_version"] = "1.0.0";

            return crow::response(result);
        });
    }

    // 保存网易云账号凭据到 settings.json（保留其余字段）
    bool save_ncm_settings(const std::string& phone, const std::string& pass, const std::string& cookie) {
        std::string raw = "{}";
        {
            std::ifstream f("settings.json");
            if (f.is_open()) { std::stringstream ss; ss << f.rdbuf(); raw = ss.str(); }
        }
        auto j = crow::json::load(raw);
        crow::json::wvalue out;
        if (j) {
            if (j.has("station_name"))     out["station_name"]     = std::string(j["station_name"].s());
            if (j.has("subtitle"))         out["subtitle"]         = std::string(j["subtitle"].s());
            if (j.has("primary_color"))    out["primary_color"]    = std::string(j["primary_color"].s());
            if (j.has("secondary_color"))  out["secondary_color"]  = std::string(j["secondary_color"].s());
            if (j.has("bg_color"))         out["bg_color"]         = std::string(j["bg_color"].s());
            if (j.has("admin_password"))   out["admin_password"]   = std::string(j["admin_password"].s());
            if (j.has("allow_guest_skip")) out["allow_guest_skip"] = j["allow_guest_skip"].b();
        }
        out["ncm_phone"]    = phone;
        out["ncm_password"] = pass;
        out["ncm_cookie"]   = cookie;

        std::ofstream f("settings.json");
        if (!f) return false;
        f << out.dump(4);
        return true;
    }

    // 文件上传处理函数：二进制安全的 multipart/form-data 解析
    crow::response handle_upload(const crow::request& req) {
        auto boundary_info = req.headers.find("Content-Type");
        if (boundary_info == req.headers.end()) return crow::response(400, "缺少Content-Type");

        std::string content_type = boundary_info->second;
        auto boundary_pos = content_type.find("boundary=");
        if (boundary_pos == std::string::npos) return crow::response(400, "无效的Content-Type");

        std::string boundary = content_type.substr(boundary_pos + 9);
        if (!boundary.empty() && boundary.front() == '"') {
            auto end_quote = boundary.find('"', 1);
            boundary = (end_quote == std::string::npos) ? boundary.substr(1) : boundary.substr(1, end_quote - 1);
        } else {
            auto semi = boundary.find(';');
            if (semi != std::string::npos) boundary = boundary.substr(0, semi);
        }
        if (boundary.empty()) return crow::response(400, "无效的Content-Type");

        const std::string& body = req.body;
        if (body.size() > Config::MAX_UPLOAD_SIZE) {
            return crow::response(413, "文件太大，最大50MB");
        }

        std::string delim = "--" + boundary;
        size_t pos = body.find(delim);
        if (pos == std::string::npos) return crow::response(400, "未找到分界符");

        while (pos < body.size()) {
            pos += delim.size();
            // 结束分界符 "--boundary--"
            if (pos + 2 <= body.size() && body[pos] == '-' && body[pos + 1] == '-') break;
            // 跳过分界符后的 CRLF
            if (pos + 2 <= body.size() && body[pos] == '\r' && body[pos + 1] == '\n') {
                pos += 2;
            } else if (pos < body.size() && body[pos] == '\n') {
                pos += 1;
            }

            // 定位部分头结束位置（空行）
            size_t headers_end = body.find("\r\n\r\n", pos);
            size_t sep_len = 4;
            if (headers_end == std::string::npos) {
                headers_end = body.find("\n\n", pos);
                sep_len = 2;
                if (headers_end == std::string::npos) break;
            }

            std::string headers = body.substr(pos, headers_end - pos);
            size_t data_start = headers_end + sep_len;

            // 找到下一个分界符，确定数据结束位置
            size_t next_delim = body.find(delim, data_start);
            if (next_delim == std::string::npos) break;
            size_t data_end = next_delim;
            if (data_end >= 2 && body[data_end - 2] == '\r' && body[data_end - 1] == '\n') {
                data_end -= 2;
            } else if (data_end >= 1 && body[data_end - 1] == '\n') {
                data_end -= 1;
            }

            // 解析 Content-Disposition：同一行既有 name 又有 filename
            auto name_pos = headers.find("name=\"");
            auto filename_pos = headers.find("filename=\"");
            if (name_pos != std::string::npos && filename_pos != std::string::npos) {
                name_pos += 6;
                auto name_end = headers.find('"', name_pos);
                std::string field_name = headers.substr(name_pos, name_end - name_pos);

                if (field_name == "file") {
                    filename_pos += 10;
                    auto filename_end = headers.find('"', filename_pos);
                    std::string filename = headers.substr(filename_pos, filename_end - filename_pos);

                    // 去掉客户端可能附带的路径，防止目录穿越
                    auto slash_pos = filename.find_last_of("/\\");
                    if (slash_pos != std::string::npos) {
                        filename = filename.substr(slash_pos + 1);
                    }

                    if (filename.empty()) {
                        return crow::response(400, "文件名为空");
                    }
                    if (data_end <= data_start) {
                        return crow::response(400, "上传的文件为空");
                    }

                    size_t file_size = data_end - data_start;

                    bool supported = false;
                    for (const auto& ext : Config::SUPPORTED_FORMATS) {
                        if (filename.size() >= ext.size() &&
                            filename.compare(filename.size() - ext.size(), ext.size(), ext) == 0) {
                            supported = true;
                            break;
                        }
                    }
                    if (!supported) {
                        std::string supported_formats;
                        for (const auto& ext : Config::SUPPORTED_FORMATS) supported_formats += ext + " ";
                        return crow::response(400, "不支持的文件格式，支持: " + supported_formats);
                    }

                    std::string filepath = "./media/" + filename;
                    std::ofstream out_file(filepath, std::ios::binary);
                    if (!out_file) return crow::response(500, "无法创建文件");
                    out_file.write(body.data() + data_start, file_size);
                    out_file.close();

                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    playlist_->push_back(filename);

                    TrackMetadata metadata = MetadataManager::extract_metadata(filepath);
                    playlist_metadata_->push_back(metadata);

                    if (playlist_->size() == 1) {
                        current_track_->store(0);
                        audio_player_->skip_current_track();
                    }

                    save_playlist_order();
                    return crow::response(200, "上传成功: " + filename);
                }
            }

            pos = next_delim;
        }

        return crow::response(400, "未找到文件数据");
    }

private:
    Config config_;
    std::unique_ptr<SessionManager> session_manager_;
    crow::App<> app_;
    std::thread thread_;
    std::atomic<bool> running_;
    
    // 其他成员变量保持不变
    std::vector<std::string>* playlist_;
    std::vector<TrackMetadata>* playlist_metadata_;
    std::atomic<size_t>* current_track_;
    StreamServer* stream_server_;
    AudioPlayer* audio_player_;
    std::mutex* playlist_mutex_;

    std::atomic<bool> dl_running_{false};
    std::mutex dl_mutex_;
    std::string dl_log_;

    // 从Cookie字符串中提取session_id
    static std::string get_session_id_from_cookies(const std::string& cookie_header) {
        if (cookie_header.empty()) return "";

        static const std::string kKey = "session_id=";
        size_t session_start = cookie_header.find(kKey);
        if (session_start == std::string::npos) return "";

        session_start += kKey.length();
        size_t session_end = cookie_header.find(';', session_start);

        std::string value = (session_end == std::string::npos)
            ? cookie_header.substr(session_start)
            : cookie_header.substr(session_start, session_end - session_start);

        // 去除首尾空白（Cookie 多值之间一般是 "; " 分隔）
        size_t first = value.find_first_not_of(" \t");
        size_t last = value.find_last_not_of(" \t");
        if (first == std::string::npos) return "";
        return value.substr(first, last - first + 1);
    }

    // 检查用户是否为管理员
    bool check_admin_auth(const std::string& session_id) {
        if (session_id.empty()) return false;
        auto session = session_manager_->get_session(session_id);
        return session && session->is_admin;
    }

    // 持有 playlist_mutex_ 时调用，将当前播放列表顺序写入 playlist_order.json
    void save_playlist_order() {
        std::ofstream f("playlist_order.json");
        if (!f.is_open()) return;
        f << "[";
        bool first = true;
        for (const auto& name : *playlist_) {
            if (!first) f << ",";
            f << "\"";
            for (char c : name) {
                if (c == '"')  f << "\\\"";
                else if (c == '\\') f << "\\\\";
                else f << c;
            }
            f << "\"";
            first = false;
        }
        f << "]";
    }
};

// 在类定义外部定义静态成员
const std::vector<std::string> WebServer::Config::SUPPORTED_FORMATS = {".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"};

// =============================================================================
// 应用程序主类
// =============================================================================
class RadioServer {
public:
    RadioServer() {
        // 初始化播放列表
        init_playlist();
    }
    
    ~RadioServer() {
        stop();
    }
    
    bool start() {
        if (running_) return false;
        
        std::cout << "\n"
            "╔══════════════════════════════════════════╗\n"
            "║        Rakuraku启动中...        ║\n"
            "╚══════════════════════════════════════════╝\n" << std::endl;
        
        // 启动各个组件
        bool success = true;
        
        stream_server_ = std::make_unique<StreamServer>(&buffer_);
        audio_player_ = std::make_unique<AudioPlayer>(&buffer_, &playlist_, &current_track_, &playlist_mutex_);
        web_server_   = std::make_unique<WebServer>(&playlist_, &playlist_metadata_, &current_track_,
                                                    stream_server_.get(), audio_player_.get(), &playlist_mutex_);
        
        success &= stream_server_->start();
        success &= audio_player_->start();
        success &= web_server_->start();
        
        if (success) {
            running_ = true;
            std::cout << "\n"
                "╔══════════════════════════════════════════╗\n"
                "║        服务器启动成功！                 ║\n"
                "║                                          ║\n"
                "║  Web界面: http://localhost:" << Config::WEB_PORT << "     ║\n"
                "║  流媒体:  http://localhost:" << Config::WEB_PORT << "/stream ║\n"
                "║                                          ║\n"
                "║  按 Ctrl+C 停止服务器                  ║\n"
                "╚══════════════════════════════════════════╝\n" << std::endl;
        }
        
        return success;
    }
    
    // 在 RadioServer 类中：
  void stop() {
    // running_ 可能已被 request_stop() / 析构前的信号处理置为 false；
    // 用独立的 stopped_ 标志保证 stop() 的清理逻辑恰好执行一次。
    bool expected = false;
    if (!stopped_.compare_exchange_strong(expected, true)) return;

    running_ = false;

    std::cout << "[System] 正在停止所有服务..." << std::endl;

    // 1. 先停 Web 服务器（它通常是阻塞主线程的元凶）
    if (web_server_) web_server_->stop();

    // 2. 停止音频播放（停止 FFmpeg 管道）
    if (audio_player_) audio_player_->stop();

    // 3. 停止流服务器（断开所有连接）
    if (stream_server_) stream_server_->stop();

    std::cout << "[System] 服务器已停止" << std::endl;
   }
    
    void wait_for_shutdown() {
        while (running_) {
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
        }
    }

    // 异步信号安全的请求停止入口：只做原子标志位切换
    void request_stop() {
        running_ = false;
    }

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

        // 扫描 media 目录，得到当前可用文件集合
        std::set<std::string> available;
        try {
            for (const auto& entry : fs::directory_iterator("./media")) {
                if (!entry.is_regular_file()) continue;
                std::string filename = entry.path().filename().string();
                std::string ext = fs::path(filename).extension();
                std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
                for (const auto& supported_ext : Config::SUPPORTED_FORMATS) {
                    if (ext == supported_ext) { available.insert(filename); break; }
                }
            }
        } catch (const fs::filesystem_error& e) {
            std::cerr << "[Init] 扫描目录时出错: " << e.what() << std::endl;
        }

        // 按已保存顺序排列，已删除的文件跳过；未在列表中的新文件按字母顺序追加末尾
        std::vector<std::string> ordered;
        std::set<std::string> seen;
        for (const auto& fname : saved_order) {
            if (available.count(fname)) { ordered.push_back(fname); seen.insert(fname); }
        }
        std::vector<std::string> new_files;
        for (const auto& fname : available) {
            if (!seen.count(fname)) new_files.push_back(fname);
        }
        std::sort(new_files.begin(), new_files.end());
        for (const auto& fname : new_files) ordered.push_back(fname);

        // 同步构建两个并行向量，保证索引一一对应
        for (const auto& fname : ordered) {
            playlist_.push_back(fname);
            playlist_metadata_.push_back(MetadataManager::extract_metadata("./media/" + fname));
        }

        if (!playlist_.empty()) {
            std::random_device rd;
            std::mt19937 gen(rd());
            std::uniform_int_distribution<> dis(0, playlist_.size() - 1);
            current_track_ = dis(gen);
        }

        std::cout << "[Init] 在 ./media/ 目录中找到 " << playlist_.size() << " 个音频文件" << std::endl;
        std::cout << "[Init] 已提取 " << playlist_metadata_.size() << " 个文件的元数据" << std::endl;
    }
    
    BroadcastBuffer buffer_{Config::BUFFER_CAPACITY};
    std::vector<std::string> playlist_; // 用于向后兼容的文件名列表
    std::vector<TrackMetadata> playlist_metadata_; // 新的元数据播放列表
    std::atomic<size_t> current_track_{0};
    mutable std::mutex playlist_mutex_;
    
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

// 信号处理器必须只使用 async-signal-safe 的操作。
// 这里只切换原子标志，让主线程自己完成清理，SIGTERM 即可优雅退出，无需 SIGKILL。
void signal_handler(int /*sig*/) {
    if (g_server_instance) {
        g_server_instance->request_stop();
    }
}

// =============================================================================
// 主函数
// =============================================================================
int main() {
    // 设置信号处理
    std::signal(SIGINT, signal_handler);
    std::signal(SIGTERM, signal_handler);
    std::signal(SIGPIPE, SIG_IGN);  // 忽略管道断开信号
    
    try {
        RadioServer server;
        g_server_instance = &server;
        
        if (!server.start()) {
            std::cerr << "[System] 服务器启动失败" << std::endl;
            return 1;
        }

        // 等待 SIGINT/SIGTERM 把 running_ 置 false
        server.wait_for_shutdown();

        std::cout << "\n[System] 收到终止信号，正在关闭服务器..." << std::endl;
        // 停止工作在主线程完成，而不是在信号处理器里（后者只是翻转原子标志）
        server.stop();

        g_server_instance = nullptr;

    } catch (const std::exception& e) {
        std::cerr << "[System] 致命错误: " << e.what() << std::endl;
        return 1;
    }
    
    std::cout << "[System] 服务器已退出" << std::endl;
    return 0;
}
