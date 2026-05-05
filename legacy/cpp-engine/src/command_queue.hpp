#pragma once

#include <mutex>
#include <vector>
#include <string>

// Thread-safe command queue — replaces Redis pub/sub for C++↔Rust communication
class CommandQueue {
public:
    void push(const std::string& cmd) {
        std::lock_guard<std::mutex> lock(mutex_);
        queue_.push_back(cmd);
    }

    bool poll(std::string& out) {
        std::lock_guard<std::mutex> lock(mutex_);
        if (queue_.empty()) return false;
        out = queue_.front();
        queue_.erase(queue_.begin());
        return true;
    }

private:
    std::mutex mutex_;
    std::vector<std::string> queue_;
};
