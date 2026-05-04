#include "audio_player.hpp"
#include "broadcast_buffer.hpp"
#include "command_queue.hpp"
#include "config.hpp"

#include <iostream>
#include <filesystem>
#include <unistd.h>
#include <fcntl.h>
#include <sys/wait.h>
#include <poll.h>
#include <csignal>
#include <cstring>
#include <iomanip>
#include <chrono>

namespace fs = std::filesystem;

AudioPlayer::AudioPlayer(BroadcastBuffer* buffer, std::vector<std::string>* playlist,
        std::atomic<size_t>* current_track, std::mutex* playlist_mutex,
        std::vector<TrackMetadata>* playlist_metadata,
        CommandQueue* cmd_queue,
        PlaybackState* state_out)
: buffer_(buffer), playlist_(playlist), current_track_(current_track),
  playlist_mutex_(playlist_mutex), playlist_metadata_(playlist_metadata),
  cmd_queue_(cmd_queue), state_out_(state_out) {}

AudioPlayer::~AudioPlayer() { stop(); }

bool AudioPlayer::start() {
    if (running_) return false;
    signal(SIGPIPE, SIG_IGN);
    running_ = true;
    thread_ = std::thread(&AudioPlayer::worker_loop, this);
    std::cout << "[Audio] Player started" << std::endl;
    return true;
}

void AudioPlayer::stop() {
    if (!running_.exchange(false)) return;
    skip_track_ = true;
    cleanup_preload();
    {
        std::lock_guard<std::mutex> lock(idle_mutex_);
        idle_cv_.notify_all();
    }
    if (thread_.joinable()) thread_.join();
    if (state_out_) {
        state_out_->song_id = 0;
        state_out_->file_path.clear();
        state_out_->position_ms = 0;
        state_out_->duration_ms = 0;
        state_out_->status = "stopped";
        state_out_->total_bytes_sent = 0;
    }
    std::cout << "[Audio] Player stopped" << std::endl;
}

void AudioPlayer::skip_current_track() { skip_track_ = true; }

void AudioPlayer::play_file(const std::string& file_path) {
    std::lock_guard<std::mutex> lock(*playlist_mutex_);
    for (size_t i = 0; i < playlist_->size(); ++i) {
        if ((*playlist_)[i] == file_path) {
            current_track_->store(i);
            skip_track_ = true;
            return;
        }
    }
    playlist_->push_back(file_path);
    playlist_metadata_->push_back(TrackMetadata("./media/" + file_path));
    current_track_->store(playlist_->size() - 1);
    skip_track_ = true;
}

void AudioPlayer::interruptible_wait(std::chrono::milliseconds dur) {
    std::unique_lock<std::mutex> lock(idle_mutex_);
    idle_cv_.wait_for(lock, dur, [this]() { return !running_.load(); });
}

void AudioPlayer::build_ffmpeg_argv(const std::string& filename, int64_t duration_ms,
                        bool is_preload,
                        std::vector<std::string>& out_strs,
                        std::vector<const char*>& out_argv) {
    out_strs.clear();
    out_strs = {"ffmpeg", "-nostdin", "-re", "-loglevel", "error"};
    out_strs.insert(out_strs.end(), {"-i", filename, "-vn",
                     "-c:a", "libmp3lame", "-b:a", "128k",
                     "-ar", "44100", "-ac", "2"});

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

void AudioPlayer::start_preload(const std::string& filename, size_t track_idx) {
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

    int flags = fcntl(preload_pipe_fd_, F_GETFL, 0);
    if (flags >= 0) fcntl(preload_pipe_fd_, F_SETFL, flags | O_NONBLOCK);

    std::cout << "[XFade] Preloading next track: " << filename << std::endl;
}

void AudioPlayer::read_preload() {
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

void AudioPlayer::drain_and_switch(int& pipe_fd, pid_t& child_pid,
                      size_t& track_idx, std::string& filename) {
    if (!preload_buffer_.empty()) {
        buffer_->push(reinterpret_cast<const char*>(preload_buffer_.data()),
                      preload_buffer_.size());
        total_bytes_sent_ += preload_buffer_.size();
        std::cout << "[XFade] Drained " << preload_buffer_.size()
                  << " bytes of preloaded audio" << std::endl;
        preload_buffer_.clear();
    }

    pipe_fd = preload_pipe_fd_;
    child_pid = preload_child_pid_;
    track_idx = preload_track_idx_;
    filename = preload_filename_;

    int flags = fcntl(pipe_fd, F_GETFL, 0);
    if (flags >= 0) fcntl(pipe_fd, F_SETFL, flags & ~O_NONBLOCK);

    preload_pipe_fd_ = -1;
    preload_child_pid_ = -1;

    std::cout << "[XFade] Switched to next track" << std::endl;
}

void AudioPlayer::cleanup_preload() {
    if (preload_pipe_fd_ >= 0) { close(preload_pipe_fd_); preload_pipe_fd_ = -1; }
    if (preload_child_pid_ > 0) {
        kill(preload_child_pid_, SIGTERM);
        waitpid(preload_child_pid_, nullptr, 0);
        preload_child_pid_ = -1;
    }
    preload_buffer_.clear();
}

void AudioPlayer::worker_loop() {
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

void AudioPlayer::play_next_track() {
    pid_t child_pid = -1;
    int pipe_fd = -1;
    size_t playlist_size = 0;

    std::string filename;
    size_t track_idx = 0;

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
        (*current_track_)++;
        interruptible_wait(std::chrono::seconds(1));
        return;
    }

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

        // Crossfade preload trigger
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

        if (preload_pipe_fd_ >= 0) {
            read_preload();
        }

        // Publish state to shared struct
        if (state_out_) {
            auto now = std::chrono::steady_clock::now();
            if (now - last_state_publish >= std::chrono::milliseconds(Config::STATE_PUBLISH_MS)) {
                last_state_publish = now;
                auto position_ms = now.time_since_epoch().count() / 1000000 - track_start_epoch_ms_;
                if (current_duration_ms_ > 0 && position_ms > current_duration_ms_) {
                    position_ms = current_duration_ms_;
                }
                state_out_->song_id = static_cast<int64_t>(track_idx);
                {
                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    if (track_idx < playlist_->size()) {
                        state_out_->file_path = playlist_->at(track_idx);
                    }
                }
                state_out_->position_ms = position_ms;
                state_out_->duration_ms = current_duration_ms_;
                state_out_->status = preload_triggered ? "crossfading" : "playing";
                state_out_->total_bytes_sent = total_bytes_sent_.load();
                state_out_->track_start_timestamp_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
                    std::chrono::system_clock::now().time_since_epoch()).count();
            }
        }

        // Process commands
        if (cmd_queue_) {
            std::string cmd_json;
            while (cmd_queue_->poll(cmd_json)) {
                // Parse simple JSON: {"type":"skip"} or {"type":"play", "file_path":"..."}
                auto extract = [&](const std::string& key) -> std::string {
                    std::string s = "\"" + key + "\":\"";
                    size_t p = cmd_json.find(s);
                    if (p == std::string::npos) {
                        s = "\"" + key + "\": \"";
                        p = cmd_json.find(s);
                    }
                    if (p == std::string::npos) return "";
                    p += s.size();
                    size_t e = cmd_json.find('"', p);
                    if (e == std::string::npos) return "";
                    return cmd_json.substr(p, e - p);
                };
                std::string cmd_type = extract("type");
                std::string cmd_file = extract("file_path");

                if ((cmd_type == "skip" || cmd_type == "next") && running_ && playlist_size > 0) {
                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    size_t sz = playlist_->size();
                    if (sz > 0) {
                        current_track_->store((current_track_->load() + 1) % sz);
                        skip_track_ = true;
                        std::cout << "[Cmd] Received skip command" << std::endl;
                    }
                } else if (cmd_type == "prev" && running_ && playlist_size > 0) {
                    std::lock_guard<std::mutex> lock(*playlist_mutex_);
                    size_t sz = playlist_->size();
                    if (sz > 0) {
                        current_track_->store((current_track_->load() + sz - 1) % sz);
                        skip_track_ = true;
                        std::cout << "[Cmd] Received prev command" << std::endl;
                    }
                } else if (cmd_type == "play" && !cmd_file.empty()) {
                    play_file(cmd_file);
                } else if (cmd_type == "stop") {
                    skip_track_ = true;
                    std::cout << "[Cmd] Received stop command" << std::endl;
                }
            }
        }
    }

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

    cleanup_preload();

    if (!skip_track_ && running_) {
        current_track_->store((current_track_->load() + 1) %
                              (playlist_size > 0 ? playlist_size : 1));
    }
    skip_track_ = false;
}
