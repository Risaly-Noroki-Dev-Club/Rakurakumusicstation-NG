#pragma once

#include <atomic>
#include <mutex>
#include <thread>
#include <condition_variable>
#include <vector>
#include <string>
#include <cstdint>

#include "metadata.hpp"

class BroadcastBuffer;
class CommandQueue;
struct PlaybackState;  // defined below

// Playback state snapshot (shared between AudioPlayer and WebServer)
struct PlaybackState {
    int64_t song_id = 0;
    std::string file_path;
    int64_t position_ms = 0;
    int64_t duration_ms = 0;
    std::string status = "stopped";
    uint64_t total_bytes_sent = 0;
    int64_t track_start_timestamp_ms = 0;
};

// 音频播放器
class AudioPlayer {
public:
    AudioPlayer(BroadcastBuffer* buffer, std::vector<std::string>* playlist,
            std::atomic<size_t>* current_track, std::mutex* playlist_mutex,
            std::vector<TrackMetadata>* playlist_metadata,
            CommandQueue* cmd_queue = nullptr,
            PlaybackState* state_out = nullptr);

    ~AudioPlayer();

    bool start();
    void stop();
    void skip_current_track();
    void play_file(const std::string& file_path);

private:
    BroadcastBuffer* buffer_;
    std::vector<std::string>* playlist_;
    std::atomic<size_t>* current_track_;
    std::mutex* playlist_mutex_;
    std::vector<TrackMetadata>* playlist_metadata_;
    CommandQueue* cmd_queue_;
    PlaybackState* state_out_;
    std::atomic<bool> running_{false};
    std::atomic<bool> skip_track_{false};
    std::thread thread_;
    std::mutex idle_mutex_;
    std::condition_variable idle_cv_;
    std::atomic<uint64_t> total_bytes_sent_{0};
    int64_t current_duration_ms_{0};
    int64_t track_start_epoch_ms_{0};

    pid_t preload_child_pid_{-1};
    int preload_pipe_fd_{-1};
    std::vector<uint8_t> preload_buffer_;
    size_t preload_track_idx_{0};
    std::string preload_filename_;

    void interruptible_wait(std::chrono::milliseconds dur);
    void build_ffmpeg_argv(const std::string& filename, int64_t duration_ms,
                           bool is_preload,
                           std::vector<std::string>& out_strs,
                           std::vector<const char*>& out_argv);
    void start_preload(const std::string& filename, size_t track_idx);
    void read_preload();
    void drain_and_switch(int& pipe_fd, pid_t& child_pid,
                          size_t& track_idx, std::string& filename);
    void cleanup_preload();
    void worker_loop();
    void play_next_track();
};
