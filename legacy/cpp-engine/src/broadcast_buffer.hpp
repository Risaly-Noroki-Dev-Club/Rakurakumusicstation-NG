#pragma once

#include <atomic>
#include <mutex>
#include <vector>
#include <condition_variable>
#include <cstring>
#include <stdexcept>

#include "config.hpp"

// 线程安全的环形缓冲区
class BroadcastBuffer {
public:
    explicit BroadcastBuffer(size_t capacity = Config::BUFFER_CAPACITY)
        : capacity_(capacity), buffer_(capacity) {
        if (capacity == 0 || (capacity & (capacity - 1)) != 0) {
            throw std::runtime_error("Capacity must be power of two");
        }
        mask_ = capacity - 1;
    }

    void push(const char* data, size_t len) {
        if (len == 0 || len > capacity_) return;
        std::lock_guard<std::mutex> lock(write_mutex_);
        size_t current_rp = consume_pos_.load(std::memory_order_relaxed);
        size_t current_wp = write_pos_.load(std::memory_order_relaxed);
        size_t used = (current_wp >= current_rp) ?
                     (current_wp - current_rp) :
                     (capacity_ - current_rp + current_wp);
        size_t free = capacity_ - used;
        if (len > free) {
            size_t advance = len - free;
            consume_pos_.store((current_rp + advance) & mask_,
                               std::memory_order_relaxed);
        }
        size_t new_wp = (current_wp + len) & mask_;
        size_t first_seg = std::min(len, capacity_ - (current_wp & mask_));
        std::memcpy(&buffer_[current_wp & mask_], data, first_seg);
        if (first_seg < len) {
            std::memcpy(&buffer_[0], data + first_seg, len - first_seg);
        }
        write_pos_.store(new_wp, std::memory_order_release);
        data_cv_.notify_all();
    }

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

    bool wait_for_data(size_t& consume_pos, int timeout_ms = 100) {
        std::unique_lock<std::mutex> lock(cv_mutex_);
        size_t wp = write_pos_.load(std::memory_order_acquire);
        if (consume_pos != wp) return true;
        return data_cv_.wait_for(lock, std::chrono::milliseconds(timeout_ms),
            [this, &consume_pos]() {
                return consume_pos != write_pos_.load(std::memory_order_acquire);
            });
    }

    size_t current_write_pos() const {
        return write_pos_.load(std::memory_order_acquire);
    }

    void wakeup_all() {
        std::lock_guard<std::mutex> lock(cv_mutex_);
        data_cv_.notify_all();
    }

private:
    const size_t capacity_;
    size_t mask_;
    std::vector<char> buffer_;
    std::atomic<size_t> write_pos_{0};
    std::atomic<size_t> consume_pos_{0};
    std::mutex write_mutex_;
    std::mutex cv_mutex_;
    std::condition_variable data_cv_;
};
