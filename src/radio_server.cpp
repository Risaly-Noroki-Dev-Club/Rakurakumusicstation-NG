#include "radio_server.hpp"
#include "stream_server.hpp"
#include "audio_player.hpp"
#include "web_server.hpp"
#include "command_queue.hpp"
#include "config.hpp"
#include "metadata.hpp"

#include <crow_all.h>
#include <iostream>
#include <fstream>
#include <sstream>
#include <filesystem>
#include <algorithm>
#include <set>
#include <random>
#include <cstring>
#include <unistd.h>

namespace fs = std::filesystem;

RadioServer::RadioServer() { init_playlist(); }

RadioServer::~RadioServer() { stop(); }

bool RadioServer::start() {
    if (running_) return false;
    std::cout << "\n"
        "╔══════════════════════════════════════════╗\n"
        "║     Rakuraku Audio Engine 启动中...     ║\n"
        "╚══════════════════════════════════════════╝\n" << std::endl;

    stream_server_ = std::make_unique<StreamServer>(&buffer_);
    audio_player_ = std::make_unique<AudioPlayer>(
        &buffer_, &playlist_, &current_track_, &playlist_mutex_,
        &playlist_metadata_, &command_queue_, &playback_state_);
    web_server_ = std::make_unique<WebServer>(
        stream_server_.get(), &command_queue_, &playback_state_);

    bool success = true;
    success &= stream_server_->start();
    success &= audio_player_->start();
    success &= web_server_->start();

    if (success) {
        running_ = true;
        std::cout << "\n"
            "╔══════════════════════════════════════════╗\n"
            "║          引擎启动成功！                 ║\n"
            "║                                          ║\n"
            "║  音频流: http://localhost:2240/stream    ║\n"
            "║  健康检查: http://localhost:2240/health  ║\n"
            "║  命令: POST http://localhost:2240/cmd   ║\n"
            "║  状态: GET http://localhost:2240/state  ║\n"
            "║                                          ║\n"
            "║  按 Ctrl+C 停止引擎                    ║\n"
            "╚══════════════════════════════════════════╝\n" << std::endl;
    }

    return success;
}

void RadioServer::stop() {
    bool expected = false;
    if (!stopped_.compare_exchange_strong(expected, true)) return;
    running_ = false;
    std::cout << "[System] 正在停止引擎..." << std::endl;
    if (web_server_) web_server_->stop();
    if (audio_player_) audio_player_->stop();
    if (stream_server_) stream_server_->stop();
    std::cout << "[System] 引擎已停止" << std::endl;
}

void RadioServer::wait_for_shutdown() {
    while (running_) {
        std::this_thread::sleep_for(std::chrono::milliseconds(200));
    }
}

void RadioServer::request_stop() { running_ = false; }

void RadioServer::init_playlist() {
    std::lock_guard<std::mutex> lock(playlist_mutex_);
    fs::create_directories("./media");

    std::vector<std::string> saved_order;
    {
        std::ifstream f("playlist_order.json");
        if (f.is_open()) {
            std::stringstream ss;
            ss << f.rdbuf();
            auto j = crow::json::load(ss.str());
            if (j && j.t() == crow::json::type::List) {
                for (size_t i = 0; i < j.size(); ++i)
                    saved_order.push_back(std::string(j[i].s()));
            }
        }
    }

    std::set<std::string> available;
    try {
        for (const auto& entry : fs::directory_iterator("./media")) {
            if (!entry.is_regular_file()) continue;
            std::string fn = entry.path().filename().string();
            std::string ext = fs::path(fn).extension();
            std::transform(ext.begin(), ext.end(), ext.begin(), ::tolower);
            for (const auto& sf : Config::SUPPORTED_FORMATS) {
                if (ext == sf) { available.insert(fn); break; }
            }
        }
    } catch (const fs::filesystem_error& e) {
        std::cerr << "[Init] 扫描目录时出错: " << e.what() << std::endl;
    }

    std::vector<std::string> ordered;
    std::set<std::string> seen;
    for (const auto& fn : saved_order) {
        if (available.count(fn)) { ordered.push_back(fn); seen.insert(fn); }
    }
    std::vector<std::string> new_files;
    for (const auto& fn : available) {
        if (!seen.count(fn)) new_files.push_back(fn);
    }
    std::sort(new_files.begin(), new_files.end());
    for (const auto& fn : new_files) ordered.push_back(fn);

    for (const auto& fn : ordered) {
        playlist_.push_back(fn);
        playlist_metadata_.push_back(MetadataManager::extract_metadata("./media/" + fn));
    }

    if (!playlist_.empty()) {
        std::random_device rd;
        std::mt19937 gen(rd());
        std::uniform_int_distribution<> dis(0, playlist_.size() - 1);
        current_track_ = dis(gen);
    }

    std::cout << "[Init] 在 ./media/ 中找到 " << playlist_.size() << " 个音频文件" << std::endl;
}
