#pragma once

#include <atomic>
#include <thread>
#include <mutex>
#include <string>
#include <vector>

class StreamServer;
class CommandQueue;
struct PlaybackState;

// Web 服务器（Crow 框架）
class WebServer {
public:
    WebServer(StreamServer* stream_server, CommandQueue* cmd_queue = nullptr,
              PlaybackState* state = nullptr,
              std::vector<std::string>* playlist = nullptr,
              std::mutex* playlist_mutex = nullptr);
    ~WebServer();

    bool start();
    void stop();

private:
    StreamServer* stream_server_;
    CommandQueue* cmd_queue_;
    PlaybackState* state_;
    std::vector<std::string>* playlist_;
    std::mutex* playlist_mutex_;
    std::thread thread_;
    std::atomic<bool> running_{false};

    void setup_routes_and_run();
};
