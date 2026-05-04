#include "web_server.hpp"
#include "stream_server.hpp"
#include "command_queue.hpp"
#include "audio_player.hpp"  // for PlaybackState
#include "config.hpp"

#include <crow_all.h>
#include <iostream>
#include <unistd.h>
#include <cstring>
#include <sstream>

WebServer::WebServer(StreamServer* stream_server, CommandQueue* cmd_queue,
                     PlaybackState* state)
    : stream_server_(stream_server), cmd_queue_(cmd_queue), state_(state) {}

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

    try {
        std::cout << "[Web] Streaming server on port " << Config::WEB_PORT << std::endl;
        app.signal_clear();
        app.port(Config::WEB_PORT).multithreaded().run();
    } catch (const std::exception& e) {
        std::cerr << "[Web] Error: " << e.what() << std::endl;
    }
    running_ = false;
}
