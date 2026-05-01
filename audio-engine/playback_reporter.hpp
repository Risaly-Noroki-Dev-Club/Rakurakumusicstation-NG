/// Redis 集成的播放状态广播器。
/// 生成 JSON 格式的播放状态消息并通过 Redis 发布。
/// 同时处理订阅命令频道以支持管理操作。

#ifndef PLAYBACK_REPORTER_HPP
#define PLAYBACK_REPORTER_HPP

#include "redis_client.hpp"
#include <string>
#include <sstream>
#include <chrono>
#include <ctime>
#include <atomic>
#include <thread>
#include <mutex>

/// 生成播放状态更新并通过 Redis 发布。
/// 跟踪当前歌曲、播放位置和字节计数。
class PlaybackReporter {
public:
    PlaybackReporter(RedisClient* redis, const std::string& channel)
        : redis_(redis), channel_(channel), running_(false) {}

    ~PlaybackReporter() {
        stop();
    }

    /// 在后台线程中启动定期报告（约每 200ms 一次）。
    void start() {
        if (running_) return;
        running_ = true;
        report_thread_ = std::thread(&PlaybackReporter::report_loop, this);
    }

    /// 停止报告。
    void stop() {
        running_ = false;
        if (report_thread_.joinable()) {
            report_thread_.join();
        }
    }

    /// 设置当前歌曲信息。在播放开始时调用。
    void set_song(int64_t song_id, const std::string& file_path,
                  int64_t duration_ms, int bitrate_kbps) {
        std::lock_guard<std::mutex> lock(mutex_);
        song_id_ = song_id;
        file_path_ = file_path;
        duration_ms_ = duration_ms;
        bitrate_kbps_ = bitrate_kbps;
        track_start_time_ = std::chrono::steady_clock::now();
        track_start_timestamp_ms_ = get_unix_ms();
        total_bytes_sent_ = 0;
        lyrics_line_ = -1;
        status_ = "playing";
        just_started_ = true;
    }

    /// 推送音频字节时更新跟踪计数。
    void add_bytes(size_t count) {
        total_bytes_sent_ += count;
    }

    /// 设置当前歌词行索引。
    void set_lyrics_line(int line_index) {
        std::lock_guard<std::mutex> lock(mutex_);
        lyrics_line_ = line_index;
    }

    /// 设置状态（playing、stopped、paused）。
    void set_status(const std::string& status) {
        std::lock_guard<std::mutex> lock(mutex_);
        status_ = status;
    }

    /// 获取估计的播放位置（毫秒）。
    int64_t get_position_ms() const {
        if (bitrate_kbps_ == 0) return 0;
        return static_cast<int64_t>(
            (total_bytes_sent_ * 8.0) / (bitrate_kbps_ * 1000) * 1000
        );
    }

private:
    static int64_t get_unix_ms() {
        auto now = std::chrono::system_clock::now();
        return std::chrono::duration_cast<std::chrono::milliseconds>(
            now.time_since_epoch()).count();
    }

    void report_loop() {
        while (running_) {
            std::this_thread::sleep_for(std::chrono::milliseconds(200));
            if (!running_) break;

            std::string json;
            {
                std::lock_guard<std::mutex> lock(mutex_);
                json = build_state_json();
            }

            if (redis_ && !json.empty()) {
                redis_->publish(channel_, json);
            }
        }
    }

    /// 构建 JSON 播放状态消息。
    std::string build_state_json() const {
        int64_t position_ms = get_position_ms();

        std::ostringstream js;
        js << "{";
        js << "\"song_id\":" << song_id_ << ",";
        js << "\"file_path\":\"" << escape_json(file_path_) << "\",";
        js << "\"position_ms\":" << position_ms << ",";
        js << "\"duration_ms\":" << duration_ms_ << ",";
        js << "\"lyrics_line\":";
        if (lyrics_line_ >= 0) {
            js << lyrics_line_;
        } else {
            js << "null";
        }
        js << ",";
        js << "\"status\":\"" << status_ << "\",";
        js << "\"total_bytes_sent\":" << total_bytes_sent_.load() << ",";
        js << "\"bitrate_kbps\":" << bitrate_kbps_ << ",";
        js << "\"track_start_timestamp_ms\":" << track_start_timestamp_ms_;
        js << "}";

        return js.str();
    }

    static std::string escape_json(const std::string& s) {
        std::string out;
        out.reserve(s.size());
        for (char c : s) {
            switch (c) {
                case '"':  out += "\\\""; break;
                case '\\': out += "\\\\"; break;
                case '\n': out += "\\n";  break;
                case '\r': out += "\\r";  break;
                case '\t': out += "\\t";  break;
                default:   out += c;
            }
        }
        return out;
    }

    RedisClient* redis_;
    std::string channel_;
    std::atomic<bool> running_{false};
    std::thread report_thread_;
    mutable std::mutex mutex_;

    // 当前歌曲状态（受 mutex_ 保护）
    int64_t song_id_ = 0;
    std::string file_path_;
    int64_t duration_ms_ = 0;
    int bitrate_kbps_ = 128;
    std::chrono::steady_clock::time_point track_start_time_;
    int64_t track_start_timestamp_ms_ = 0;
    std::atomic<uint64_t> total_bytes_sent_{0};
    int lyrics_line_ = -1;
    std::string status_ = "stopped";
    bool just_started_ = false;
};

#endif // PLAYBACK_REPORTER_HPP
