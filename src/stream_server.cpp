#include "stream_server.hpp"
#include "client_connection.hpp"
#include "broadcast_buffer.hpp"

#include <algorithm>
#include <iostream>
#include <unistd.h>

StreamServer::StreamServer(BroadcastBuffer* buffer) : buffer_(buffer) {}

StreamServer::~StreamServer() { stop(); }

void StreamServer::add_client(int fd) {
    std::lock_guard<std::mutex> lock(mutex_);
    if (clients_.size() >= Config::MAX_CONNECTIONS) {
        close(fd);
        return;
    }
    clients_.emplace_back(std::make_unique<ClientConnection>(fd, buffer_));
    auto* cc = clients_.back().get();
    if (!cc->send_header()) {
        cc->close_socket();
        clients_.pop_back();
        return;
    }
    new_clients_.push_back(cc);
}

bool StreamServer::start() {
    if (running_) return false;
    epoll_fd_ = epoll_create1(0);
    if (epoll_fd_ < 0) {
        std::cerr << "[Stream] epoll_create1 failed" << std::endl;
        return false;
    }
    running_ = true;
    thread_ = std::thread(&StreamServer::worker_loop, this);
    std::cout << "[Stream] Server started" << std::endl;
    return true;
}

void StreamServer::stop() {
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

size_t StreamServer::client_count() const {
    std::lock_guard<std::mutex> lock(mutex_);
    return clients_.size();
}

void StreamServer::worker_loop() {
    struct epoll_event ev, events[Config::MAX_EVENTS];
    while (running_) {
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

        {
            std::lock_guard<std::mutex> lock(mutex_);
            for (auto& c : clients_) {
                if (!c->is_shutdown() && !c->send_audio()) {
                    c->close_socket();
                }
            }
            clients_.erase(
                std::remove_if(clients_.begin(), clients_.end(),
                    [](const auto& c) { return c->is_shutdown(); }),
                clients_.end());
        }
    }
}
