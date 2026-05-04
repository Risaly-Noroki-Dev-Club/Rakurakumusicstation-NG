#pragma once

#include <atomic>
#include <memory>
#include <mutex>
#include <vector>
#include <string>

#include "broadcast_buffer.hpp"
#include "command_queue.hpp"
#include "audio_player.hpp"  // for PlaybackState
#include "metadata.hpp"

class StreamServer;
class AudioPlayer;
class WebServer;

// 主控制器
class RadioServer {
public:
    RadioServer();
    ~RadioServer();

    bool start();
    void stop();
    void wait_for_shutdown();
    void request_stop();

private:
    void init_playlist();

    BroadcastBuffer buffer_{Config::BUFFER_CAPACITY};
    std::vector<std::string> playlist_;
    std::vector<TrackMetadata> playlist_metadata_;
    std::atomic<size_t> current_track_{0};
    mutable std::mutex playlist_mutex_;

    CommandQueue command_queue_;
    PlaybackState playback_state_;

    std::unique_ptr<StreamServer> stream_server_;
    std::unique_ptr<AudioPlayer> audio_player_;
    std::unique_ptr<WebServer> web_server_;

    std::atomic<bool> running_{false};
    std::atomic<bool> stopped_{false};
};
