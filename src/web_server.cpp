#include "web_server.hpp"
#include "stream_server.hpp"
#include "command_queue.hpp"
#include "audio_player.hpp"  // for PlaybackState
#include "config.hpp"

#include <crow_all.h>
#include <iostream>
#include <fstream>
#include <unistd.h>
#include <cstring>
#include <sstream>
#include <algorithm>
#include <cctype>

WebServer::WebServer(StreamServer* stream_server, CommandQueue* cmd_queue,
                     PlaybackState* state,
                     std::vector<std::string>* playlist,
                     std::mutex* playlist_mutex)
    : stream_server_(stream_server), cmd_queue_(cmd_queue), state_(state),
      playlist_(playlist), playlist_mutex_(playlist_mutex) {}

WebServer::~WebServer() { stop(); }

bool WebServer::start() {
    if (running_) return false;
    running_ = true;
    thread_ = std::thread(&WebServer::setup_routes_and_run, this);
    return true;
}

void WebServer::stop() {
    if (!running_.exchange(false)) return;
    if (thread_.joinable()) thread_.join();
    std::cout << "[Web] Server stopped" << std::endl;
}

void WebServer::setup_routes_and_run() {
    crow::SimpleApp app;
    StreamServer* ss = stream_server_;
    CommandQueue* cq = cmd_queue_;
    PlaybackState* ps = state_;

    // 音频流端点
    CROW_ROUTE(app, "/stream")([ss](const crow::request&, crow::response& res) {
        if (!res.get_socket_fd_helper_) {
            res.code = 500;
            res.end("stream unavailable");
            return;
        }
        int crow_fd = res.get_socket_fd_helper_();
        if (crow_fd < 0) {
            res.code = 500;
            res.end("invalid socket");
            return;
        }
        int fd = ::dup(crow_fd);
        if (fd < 0) {
            res.code = 500;
            res.end("dup failed");
            return;
        }
        res.take_over();
        ss->add_client(fd);
    });

    // 健康检查端点
    CROW_ROUTE(app, "/health")([ss]() {
        crow::json::wvalue result;
        result["status"] = "ok";
        result["service"] = "rakuraku-audio-engine";
        result["clients"] = static_cast<int>(ss->client_count());
        return crow::response(result);
    });

    // 命令端点（接收 Rust 后端的 skip/prev/play/stop 命令）
    CROW_ROUTE(app, "/command").methods("POST"_method)([cq](const crow::request& req) {
        if (!cq) {
            return crow::response(503, "command queue not available");
        }
        cq->push(req.body);
        return crow::response(200, "ok");
    });

    // 状态端点（Rust 后端轮询播放状态）
    CROW_ROUTE(app, "/state")([ps]() {
        crow::json::wvalue result;
        if (ps) {
            result["song_id"] = ps->song_id;
            result["file_path"] = ps->file_path;
            result["position_ms"] = ps->position_ms;
            result["duration_ms"] = ps->duration_ms;
            result["status"] = ps->status;
            result["total_bytes_sent"] = static_cast<int64_t>(ps->total_bytes_sent);
            result["track_start_timestamp_ms"] = ps->track_start_timestamp_ms;
            result["lyrics_line"] = nullptr;
            result["bitrate_kbps"] = 128;
        } else {
            result["song_id"] = 0;
            result["status"] = "unavailable";
        }
        return crow::response(result);
    });

    // 文件端点（支持 Range 请求，用于前端推文件模式）
    CROW_ROUTE(app, "/file/<int>")([playlist=playlist_, playlist_mutex=playlist_mutex_](const crow::request& req, crow::response& res, int song_id) {
        // 查找文件路径
        std::string file_path;
        {
            std::lock_guard<std::mutex> lock(*playlist_mutex);
            if (song_id < 0 || song_id >= static_cast<int>(playlist->size())) {
                res.code = 404;
                res.end("song not found");
                return;
            }
            file_path = "./media/" + playlist->at(song_id);
        }

        // 打开文件
        std::ifstream file(file_path, std::ios::binary | std::ios::ate);
        if (!file.is_open()) {
            res.code = 404;
            res.end("file not found");
            return;
        }

        std::streamsize file_size = file.tellg();
        file.seekg(0, std::ios::beg);

        // 检测文件扩展名设置 Content-Type
        std::string ext;
        auto dot_pos = file_path.rfind('.');
        if (dot_pos != std::string::npos) {
            ext = file_path.substr(dot_pos);
            for (char& c : ext) c = std::tolower(c);
        }

        std::string content_type = "application/octet-stream";
        if (ext == ".mp3")  content_type = "audio/mpeg";
        else if (ext == ".flac") content_type = "audio/flac";
        else if (ext == ".ogg")  content_type = "audio/ogg";
        else if (ext == ".wav")  content_type = "audio/wav";
        else if (ext == ".m4a")  content_type = "audio/mp4";
        else if (ext == ".aac")  content_type = "audio/aac";

        // 处理 Range 请求
        std::string range_header = req.get_header_value("Range");
        if (!range_header.empty() && range_header.find("bytes=") == 0) {
            std::string range_val = range_header.substr(6);
            size_t dash = range_val.find('-');
            int64_t start = 0, end = file_size - 1;

            try {
                if (dash == 0) {
                    // bytes=-N → 最后 N 字节
                    int64_t suffix = std::stoll(range_val.substr(1));
                    start = std::max(int64_t(0), file_size - suffix);
                } else if (dash == range_val.length() - 1) {
                    // bytes=N-
                    start = std::stoll(range_val.substr(0, dash));
                } else {
                    // bytes=N-M
                    start = std::stoll(range_val.substr(0, dash));
                    end = std::stoll(range_val.substr(dash + 1));
                    if (end >= file_size) end = file_size - 1;
                }
            } catch (...) {
                res.code = 416;
                res.end("invalid range");
                return;
            }

            if (start >= file_size) {
                res.code = 416;
                res.end("range not satisfiable");
                return;
            }

            int64_t content_length = end - start + 1;
            std::vector<char> buffer(content_length);
            file.seekg(start);
            file.read(buffer.data(), content_length);

            res.code = 206;
            res.add_header("Content-Range",
                "bytes " + std::to_string(start) + "-" + std::to_string(end) +
                "/" + std::to_string(file_size));
            res.add_header("Content-Length", std::to_string(content_length));
            res.add_header("Content-Type", content_type);
            res.add_header("Accept-Ranges", "bytes");
            res.add_header("Access-Control-Allow-Origin", "*");
            res.write(std::string(buffer.data(), buffer.size()));
            res.end();
        } else {
            // 完整文件响应
            std::vector<char> buffer(file_size);
            file.read(buffer.data(), file_size);

            res.add_header("Content-Length", std::to_string(file_size));
            res.add_header("Content-Type", content_type);
            res.add_header("Accept-Ranges", "bytes");
            res.add_header("Access-Control-Allow-Origin", "*");
            res.write(std::string(buffer.data(), buffer.size()));
            res.end();
        }
    });

    try {
        std::cout << "[Web] Streaming server on port " << Config::WEB_PORT << std::endl;
        app.signal_clear();
        app.port(Config::WEB_PORT).multithreaded().run();
    } catch (const std::exception& e) {
        std::cerr << "[Web] Error: " << e.what() << std::endl;
    }
    running_ = false;
}
