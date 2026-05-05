#pragma once

#include <atomic>
#include <unistd.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <netinet/in.h>

#include "config.hpp"

// 前向声明
class BroadcastBuffer;

// 单个流媒体客户端连接
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

    bool send_audio();

private:
    int fd_;
    BroadcastBuffer* buffer_;
    size_t consume_pos_;
    std::atomic<bool> shutdown_{false};
    bool header_sent_{false};
};
