#pragma once

#include <memory>
#include <deque>
#include <vector>
#include <mutex>
#include <thread>
#include <atomic>
#include <sys/epoll.h>

#include "config.hpp"

class BroadcastBuffer;
class ClientConnection;

// epoll 驱动的流媒体服务器
class StreamServer {
public:
    explicit StreamServer(BroadcastBuffer* buffer);
    ~StreamServer();

    void add_client(int fd);
    bool start();
    void stop();
    size_t client_count() const;

private:
    BroadcastBuffer* buffer_;
    int epoll_fd_{-1};
    std::atomic<bool> running_{false};
    std::thread thread_;
    mutable std::mutex mutex_;
    std::deque<std::unique_ptr<ClientConnection>> clients_;
    std::vector<ClientConnection*> new_clients_;

    void worker_loop();
};
