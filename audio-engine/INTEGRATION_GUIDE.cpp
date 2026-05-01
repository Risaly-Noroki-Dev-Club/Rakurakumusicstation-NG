// =============================================================================
// radioserver.cpp — Redis + 文件服务器 + 播放报告器的集成指南
// =============================================================================
//
// 本文档记录了将新功能集成到现有 radioserver.cpp 中所需的精确更改。
// 请逐步应用这些更改。
//
// ─── 第 1 部分：文件顶部新增 #include ──────────────────────────────────────
//
// 在现有 include 之后添加（大约第 33 行附近）：

// #include "redis_client.hpp"
// #include "file_server.hpp"
// #include "playback_reporter.hpp"
// #include "config.hpp"
// #include <nlohmann/json.hpp>  // 或使用已有的 crow::json

// ─── 第 2 部分：Config 命名空间扩展（大约第 37 行附近）──────────────────
//
// 添加到 Config 命名空间：

/*
namespace Config {
    // ... 现有常量 ...

    // 新增：文件服务器常量
    constexpr size_t MAX_UPLOAD_SIZE = 50 * 1024 * 1024;

    // 新增：支持的格式
    const std::vector<std::string> SUPPORTED_FORMATS = {
        ".mp3", ".wav", ".flac", ".ogg", ".m4a", ".aac"
    };

    // 新增：Redis 配置（从配置文件或环境变量加载）
    inline std::string REDIS_HOST = "127.0.0.1";
    inline int REDIS_PORT = 6379;
    inline std::string PLAYBACK_CHANNEL = "playback_state";
    inline std::string COMMAND_CHANNEL = "command";
    inline std::string MEDIA_PATH = "./media";
}
*/

// ─── 第 3 部分：修改 AudioPlayer 类（大约第 438 行附近）─────────────────
//
// 添加以下成员并修改构造函数/play_next_track：

/*
class AudioPlayer {
public:
    // 修改后的构造函数：添加 RedisClient* 和 PlaybackReporter* 参数
    AudioPlayer(BroadcastBuffer* buffer, std::vector<std::string>* playlist,
            std::atomic<size_t>* current_track, std::mutex* playlist_mutex,
            RedisClient* redis = nullptr,
            PlaybackReporter* reporter = nullptr)
    : buffer_(buffer), playlist_(playlist), current_track_(current_track),
      playlist_mutex_(playlist_mutex), redis_(redis), reporter_(reporter),
      running_(false), skip_track_(false) {}
    // ... 其余构造函数代码 ...

    // ... 现有方法 ...

    // 新增：播放指定文件（来自 Redis 命令或直接请求）
    void play_file(const std::string& file_path, int64_t song_id) {
        std::lock_guard<std::mutex> lock(playback_state_mutex_);
        requested_file_ = file_path;
        requested_song_id_ = song_id;
        play_requested_file_ = true;
        skip_track_ = true; // 中断当前播放
    }

    // 新增：获取当前歌曲信息以用于报告
    int64_t get_current_song_id() const {
        std::lock_guard<std::mutex> lock(playback_state_mutex_);
        return current_song_id_;
    }
    std::string get_current_file_path() const {
        std::lock_guard<std::mutex> lock(playback_state_mutex_);
        return current_file_path_;
    }

// 修改后的 play_next_track()：
// 在从 FFmpeg 管道读取之后、调用 buffer_->push() 之前：
//
//    buffer_->push(buffer, bytes);
//    if (reporter_) {                            // <-- 新增
//        reporter_->add_bytes(bytes);             // <-- 新增
//    }                                             // <-- 新增
//
// 在 fork() 成功之后（child_pid > 0）、进入 poll 循环之前：
//
//    // 设置播放报告器                                     // <-- 新增
//    if (reporter_) {                                    // <-- 新增
//        int64_t song_id = 0;                            // <-- 新增
//        int duration_ms = 0;                            // <-- 新增
//        // 从元数据获取时长和 song_id                        // <-- 新增
//        reporter_->set_song(song_id, filename,          // <-- 新增
//                          duration_ms * 1000, 128);      // <-- 新增
//    }                                                   // <-- 新增
//
// 在函数末尾，当曲目完成或被跳过时：
//    if (reporter_) {                                    // <-- 新增
//        reporter_->set_status("stopped");               // <-- 新增
//    }                                                   // <-- 新增
//
// 在现有的 skip_track_ 检查之后（第 669 行）：
//    if (!skip_track_ && running_) {
//        // 通过 Redis：发布 track_complete 事件              // <-- 新增
//        if (redis_) {                                   // <-- 新增
//            std::string event = "{\"type\":\"track_end\"}"; // <-- 新增
//            redis_->publish("queue_event", event);      // <-- 新增
//        }                                               // <-- 新增
//        current_track_->store(                          // 现有代码
//            (current_track_->load() + 1) %              // 现有代码
//            (playlist_size > 0 ? playlist_size : 1));   // 现有代码
//    }

private:
    // ... 现有成员 ...
    BroadcastBuffer* buffer_;
    std::vector<std::string>* playlist_;
    std::atomic<size_t>* current_track_;
    std::mutex* playlist_mutex_;
    std::atomic<bool> running_{false};
    std::atomic<bool> skip_track_{false};
    std::thread thread_;
    std::mutex idle_mutex_;
    std::condition_variable idle_cv_;

    // 新增：Redis 集成成员
    RedisClient* redis_;
    PlaybackReporter* reporter_;

    // 新增：请求的文件播放状态
    mutable std::mutex playback_state_mutex_;
    bool play_requested_file_ = false;
    std::string requested_file_;
    int64_t requested_song_id_ = 0;
    int64_t current_song_id_ = 0;
    std::string current_file_path_;
};

*/

// ─── 第 4 部分：Redis 命令订阅器（新增独立函数）─────────────────────────
//
// 添加此函数以处理来自 Redis 的传入命令：

/*
void run_redis_command_subscriber(RedisClient* redis, AudioPlayer* audio_player) {
    redis->start_subscribe(
        {Config::COMMAND_CHANNEL},
        [audio_player](const std::string& channel, const std::string& message) {
            // 解析 JSON 命令
            auto j = crow::json::load(message);
            if (!j) {
                std::cerr << "[Redis] 无效命令：" << message << std::endl;
                return;
            }

            std::string cmd_type;
            if (j.has("type")) {
                cmd_type = j["type"].s();
            }

            if (cmd_type == "skip") {
                std::cout << "[Redis] 收到跳过命令" << std::endl;
                audio_player->skip_current_track();
            }
            else if (cmd_type == "play") {
                std::string file_path;
                int64_t song_id = 0;
                if (j.has("file_path")) file_path = j["file_path"].s();
                if (j.has("song_id")) song_id = static_cast<int64_t>(j["song_id"].i());

                if (!file_path.empty()) {
                    std::string full_path = Config::MEDIA_PATH + "/" + file_path;
                    std::cout << "[Redis] 收到播放命令：" << full_path << std::endl;
                    audio_player->play_file(full_path, song_id);
                }
            }
            else if (cmd_type == "stop") {
                // 如需处理停止操作
            }
        }
    );
}
*/

// ─── 第 5 部分：WebServer::setup_routes() 中的文件服务器路由（大约第 853 行）─
//
// 添加以下新路由：

/*
// 新增：push-file 模式的文件服务端点
CROW_ROUTE(app_, "/file/<int>")([this](int song_id, const crow::request& req) {
    // 根据 song ID 查找文件路径
    std::string file_path;
    {
        std::lock_guard<std::mutex> lock(*playlist_mutex_);
        size_t idx = static_cast<size_t>(song_id);
        if (idx < playlist_->size()) {
            file_path = "./media/" + playlist_->at(idx);
        }
    }
    if (file_path.empty()) {
        return crow::response(404, "未找到歌曲");
    }

    // 从 Crow 获取原始 socket fd
    if (!req.get_socket_fd_helper_) {
        return crow::response(500, "文件流传输不可用");
    }
    int crow_fd = req.get_socket_fd_helper_();
    int fd = ::dup(crow_fd);
    if (fd < 0) {
        return crow::response(500, "dup 失败");
    }

    // 获取 Range 头部（如果存在）
    std::string range_header;
    auto range_it = req.headers.find("Range");
    if (range_it != req.headers.end()) {
        range_header = range_it->second;
    }

    // 提供带 Range 支持的文件服务
    req.take_over();
    serve_file_range(fd, file_path, range_header);
    ::close(fd);
});

// 新增：歌词文件端点
CROW_ROUTE(app_, "/api/lyrics/file/<int>")([this](int song_id) {
    std::string file_path;
    {
        std::lock_guard<std::mutex> lock(*playlist_mutex_);
        size_t idx = static_cast<size_t>(song_id);
        if (idx < playlist_->size()) {
            // 查找音频文件同名的 .lrc 文件
            std::string audio_path = "./media/" + playlist_->at(idx);
            fs::path lrc_path = fs::path(audio_path).replace_extension(".lrc");
            if (fs::exists(lrc_path)) {
                file_path = lrc_path.string();
            }
        }
    }
    if (file_path.empty()) {
        return crow::response(404, "未找到歌词文件");
    }

    std::ifstream f(file_path);
    if (!f.is_open()) return crow::response(404, "无法读取歌词");
    std::stringstream ss;
    ss << f.rdbuf();
    return crow::response(ss.str());
});
*/

// ─── 第 6 部分：修改 RadioServer 类（大约第 1818 行附近）───────────────
//
// 将 Redis 客户端和报告器添加为顶层成员：

/*
class RadioServer {
public:
    RadioServer() {
        // 加载配置
        config_ = AudioEngineConfig::load("audio_engine.toml");

        // 初始化 Redis
        redis_client_ = std::make_unique<RedisClient>(
            config_.redis_host, config_.redis_port, config_.redis_password);

        if (!redis_client_->connect()) {
            std::cerr << "[System] 警告：无法连接到 Redis。"
                      << "播放状态和命令将不可用。" << std::endl;
            redis_client_.reset(); // 若 Redis 不可用，则继续运行
        }

        // 初始化播放报告器（如果 Redis 可用）
        if (redis_client_) {
            playback_reporter_ = std::make_unique<PlaybackReporter>(
                redis_client_.get(), config_.playback_channel);
            playback_reporter_->start();
        }

        // 初始化播放列表
        init_playlist();
    }

    // ... 现有方法 ...

    bool start() {
        // ... 现有启动代码 ...

        // 修改：将 Redis 和报告器传递给 AudioPlayer
        audio_player_ = std::make_unique<AudioPlayer>(
            &buffer_, &playlist_, &current_track_, &playlist_mutex_,
            redis_client_.get(), playback_reporter_.get());  // <-- 新增参数

        // 启动 Redis 命令订阅器（如果 Redis 可用）
        if (redis_client_) {
            run_redis_command_subscriber(redis_client_.get(), audio_player_.get());
        }

        // ... 其余现有启动代码 ...
    }

    void stop() {
        // ... 现有停止代码 ...

        // 停止播放报告器
        if (playback_reporter_) {
            playback_reporter_->stop();
        }

        // 断开 Redis
        if (redis_client_) {
            redis_client_->disconnect();
        }
    }

private:
    // ... 现有成员 ...
    BroadcastBuffer buffer_{Config::BUFFER_CAPACITY};
    std::vector<std::string> playlist_;
    std::vector<TrackMetadata> playlist_metadata_;
    std::atomic<size_t> current_track_{0};
    mutable std::mutex playlist_mutex_;
    std::unique_ptr<StreamServer> stream_server_;
    std::unique_ptr<AudioPlayer> audio_player_;
    std::unique_ptr<WebServer> web_server_;
    std::atomic<bool> running_{false};
    std::atomic<bool> stopped_{false};

    // 新增成员
    AudioEngineConfig config_;
    std::unique_ptr<RedisClient> redis_client_;
    std::unique_ptr<PlaybackReporter> playback_reporter_;
};
*/

// ─── 第 7 部分：WebServer::setup_routes() 中的歌词 API 端点 ────────────
//
// 修改 /api/lyrics/<int> 处现有的歌词端点（第 1244 行）：
// 现有端点返回嵌入的歌词文本。
// 如果 LRC 文件存在，新增一个返回 LRC 文件内容的端点：

/*
// 修改后的：歌词端点现在也会尝试读取 LRC 文件
CROW_ROUTE(app_, "/api/lyrics/<int>")([this](int idx) {
    std::lock_guard<std::mutex> lock(*playlist_mutex_);

    size_t index = static_cast<size_t>(idx);
    if (index >= playlist_metadata_->size()) {
        return crow::response(404, "索引超出范围");
    }

    const TrackMetadata& metadata = playlist_metadata_->at(index);

    // 优先尝试 LRC 文件
    fs::path audio_path = fs::path("media") / metadata.filename;
    fs::path lrc_path = audio_path.replace_extension(".lrc");
    if (fs::exists(lrc_path)) {
        std::ifstream f(lrc_path);
        if (f.is_open()) {
            std::stringstream ss;
            ss << f.rdbuf();
            return crow::response(ss.str());
        }
    }

    // 回退到嵌入的歌词
    if (!metadata.lyrics.empty()) {
        return crow::response(metadata.lyrics);
    }

    return crow::response(404, "无歌词");
});
*/
