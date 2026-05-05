#pragma once

#include <vector>
#include <string>

// 配置常量
namespace Config {
    constexpr int WEB_PORT = 2240;
    constexpr int REDIS_PORT = 6379;
    const char* const REDIS_HOST = "127.0.0.1";
    constexpr size_t BUFFER_CAPACITY = 512 * 1024;
    constexpr size_t AUDIO_CHUNK_SIZE = 16384;
    constexpr int EPOLL_TIMEOUT_MS = 100;
    constexpr int POLL_TIMEOUT_MS = 200;
    constexpr int MAX_EVENTS = 1024;
    constexpr int MAX_CONNECTIONS = 1024;
    constexpr int STATE_PUBLISH_MS = 500;
    constexpr int CROSSFADE_SECONDS = 3;
    inline const std::vector<std::string> SUPPORTED_FORMATS = {
        ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"
    };
}
