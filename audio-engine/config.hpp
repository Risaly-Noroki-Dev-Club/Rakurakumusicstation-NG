/// C++ 音频引擎的配置管理器。
/// 从 audio_engine.toml（TOML 子集解析器）或环境变量中加载设置。

#ifndef AUDIO_CONFIG_HPP
#define AUDIO_CONFIG_HPP

#include <string>
#include <fstream>
#include <sstream>
#include <iostream>
#include <cstdlib>
#include <map>

struct AudioEngineConfig {
    // Redis 设置
    std::string redis_host = "127.0.0.1";
    int redis_port = 6379;
    std::string redis_password;
    std::string playback_channel = "playback_state";
    std::string command_channel = "command";
    std::string queue_channel = "queue_event";

    // 服务器设置
    int stream_port = 2240;
    int file_port = 2240;       // 默认与 stream_port 相同
    int web_port = 2240;
    int buffer_capacity = 512 * 1024;  // 512KB
    int audio_chunk_size = 16384;      // 16KB
    int max_connections = 1024;

    // 音频设置
    std::string ffmpeg_path = "ffmpeg";
    std::string media_path = "./media";
    int bitrate_kbps = 128;
    int sample_rate = 44100;
    int channels = 2;

    // 文件扩展名
    std::vector<std::string> supported_formats = {
        ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"
    };

    /// 从类 TOML 配置文件加载。
    static AudioEngineConfig load(const std::string& path) {
        AudioEngineConfig cfg;
        std::ifstream f(path);
        if (!f.is_open()) {
            std::cerr << "[Config] 无法打开 " << path
                      << "，使用默认值 + 环境变量" << std::endl;
            cfg.apply_env_overrides();
            return cfg;
        }

        std::map<std::string, std::string> kv;
        std::string line;
        std::string current_section;

        while (std::getline(f, line)) {
            // 去除空白
            size_t start = line.find_first_not_of(" \t\r");
            if (start == std::string::npos) continue;
            size_t end = line.find_last_not_of(" \t\r");
            line = line.substr(start, end - start + 1);

            // 跳过注释
            if (line[0] == '#') continue;

            // 节头部
            if (line[0] == '[' && line.back() == ']') {
                current_section = line.substr(1, line.size() - 2);
                continue;
            }

            // Key = value
            auto eq = line.find('=');
            if (eq == std::string::npos) continue;

            std::string key = trim(line.substr(0, eq));
            std::string val = trim(line.substr(eq + 1));

            // 去除 val 的引号
            if (val.size() >= 2 && val.front() == '"' && val.back() == '"') {
                val = val.substr(1, val.size() - 2);
            }

            if (!current_section.empty()) {
                kv[current_section + "." + key] = val;
            } else {
                kv[key] = val;
            }
        }

        // 映射配置值
        auto get_str = [&](const std::string& key, const std::string& def) -> std::string {
            auto it = kv.find(key);
            return it != kv.end() ? it->second : def;
        };
        auto get_int = [&](const std::string& key, int def) -> int {
            auto it = kv.find(key);
            if (it != kv.end()) {
                try { return std::stoi(it->second); }
                catch (...) { return def; }
            }
            return def;
        };

        cfg.redis_host = get_str("redis.host", cfg.redis_host);
        cfg.redis_port = get_int("redis.port", cfg.redis_port);
        cfg.redis_password = get_str("redis.password", cfg.redis_password);
        cfg.playback_channel = get_str("redis.playback_channel", cfg.playback_channel);
        cfg.command_channel = get_str("redis.command_channel", cfg.command_channel);
        cfg.queue_channel = get_str("redis.queue_channel", cfg.queue_channel);

        cfg.stream_port = get_int("server.stream_port", cfg.stream_port);
        cfg.web_port = get_int("server.web_port", cfg.web_port);
        cfg.buffer_capacity = get_int("server.buffer_capacity", cfg.buffer_capacity);
        cfg.max_connections = get_int("server.max_connections", cfg.max_connections);

        cfg.ffmpeg_path = get_str("audio.ffmpeg_path", cfg.ffmpeg_path);
        cfg.media_path = get_str("audio.media_path", cfg.media_path);
        cfg.bitrate_kbps = get_int("audio.bitrate_kbps", cfg.bitrate_kbps);
        cfg.sample_rate = get_int("audio.sample_rate", cfg.sample_rate);
        cfg.channels = get_int("audio.channels", cfg.channels);

        cfg.apply_env_overrides();
        return cfg;
    }

    /// 通过环境变量覆盖（AUDIO_* 前缀）。
    void apply_env_overrides() {
        auto env_str = [](const char* name, std::string& target) {
            const char* val = std::getenv(name);
            if (val) target = val;
        };
        auto env_int = [](const char* name, int& target) {
            const char* val = std::getenv(name);
            if (val) { try { target = std::stoi(val); } catch (...) {} }
        };

        env_str("AUDIO_REDIS_HOST", redis_host);
        env_int("AUDIO_REDIS_PORT", redis_port);
        env_str("AUDIO_REDIS_PASSWORD", redis_password);
        env_int("AUDIO_STREAM_PORT", stream_port);
        env_int("AUDIO_WEB_PORT", web_port);
        env_str("AUDIO_MEDIA_PATH", media_path);
        env_int("AUDIO_BITRATE_KBPS", bitrate_kbps);
    }

private:
    static std::string trim(const std::string& s) {
        size_t start = s.find_first_not_of(" \t\r\n");
        if (start == std::string::npos) return "";
        size_t end = s.find_last_not_of(" \t\r\n");
        return s.substr(start, end - start + 1);
    }
};

#endif // AUDIO_CONFIG_HPP
