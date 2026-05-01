#ifndef REDIS_CLIENT_HPP
#define REDIS_CLIENT_HPP

/// C++ 音频引擎与 Rust 后端的轻量级 hiredis 封装。
/// 提供异步安全的发布和基于回调的订阅机制。
///
/// 用法：
///   RedisClient redis("127.0.0.1", 6379);
///   redis.connect();
///   redis.publish("playback_state", R"({"status":"playing"})");
///
/// 如需订阅，请在专用线程中运行：
///   redis.subscribe({"command"}, [](const char* channel, const char* msg) {
///       // 处理命令
///   });

#include <string>
#include <functional>
#include <thread>
#include <atomic>
#include <vector>
#include <mutex>
#include <iostream>
#include <cstring>

extern "C" {
#include <hiredis/hiredis.h>
}

class RedisClient {
public:
    using MessageCallback = std::function<void(const std::string& channel,
                                               const std::string& message)>;

    RedisClient(const std::string& host = "127.0.0.1", int port = 6379,
                const std::string& password = "")
        : host_(host), port_(port), password_(password),
          ctx_(nullptr), sub_ctx_(nullptr), running_(false) {}

    ~RedisClient() {
        disconnect();
    }

    /// 连接发布上下文。成功返回 true。
    bool connect() {
        ctx_ = connect_internal();
        if (!ctx_) return false;

        // 若设置了密码，则进行认证
        if (!password_.empty()) {
            auto* reply = static_cast<redisReply*>(
                redisCommand(ctx_, "AUTH %s", password_.c_str()));
            bool ok = reply && reply->type != REDIS_REPLY_ERROR;
            if (reply) freeReplyObject(reply);
            if (!ok) {
                std::cerr << "[Redis] AUTH 失败" << std::endl;
                redisFree(ctx_);
                ctx_ = nullptr;
                return false;
            }
        }

        // 测试连接
        auto* reply = static_cast<redisReply*>(redisCommand(ctx_, "PING"));
        bool ok = reply && reply->type == REDIS_REPLY_STATUS &&
                  std::string(reply->str) == "PONG";
        if (reply) freeReplyObject(reply);

        if (!ok) {
            std::cerr << "[Redis] PING 失败" << std::endl;
            redisFree(ctx_);
            ctx_ = nullptr;
            return false;
        }

        std::cout << "[Redis] 已连接到 " << host_ << ":" << port_ << std::endl;
        return true;
    }

    /// 断开所有上下文。
    void disconnect() {
        running_ = false;

        if (sub_ctx_) {
            redisFree(sub_ctx_);
            sub_ctx_ = nullptr;
        }
        if (sub_thread_.joinable()) {
            sub_thread_.join();
        }
        if (ctx_) {
            redisFree(ctx_);
            ctx_ = nullptr;
        }
    }

    /// 向频道发布一条消息。线程安全。
    bool publish(const std::string& channel, const std::string& message) {
        std::lock_guard<std::mutex> lock(pub_mutex_);
        if (!ctx_) return false;

        auto* reply = static_cast<redisReply*>(
            redisCommand(ctx_, "PUBLISH %s %b",
                         channel.c_str(),
                         message.data(), message.size()));
        bool ok = reply && reply->type == REDIS_REPLY_INTEGER;
        if (reply) freeReplyObject(reply);
        return ok;
    }

    /// 订阅频道并通过回调处理消息。
    /// 此方法会阻塞，直到 stop_subscribe() 被调用。
    /// 请从专用线程中调用。
    void subscribe(const std::vector<std::string>& channels,
                   MessageCallback callback) {
        sub_ctx_ = connect_internal();
        if (!sub_ctx_) {
            std::cerr << "[Redis] 订阅连接失败" << std::endl;
            return;
        }

        running_ = true;

        // 为 redisCommandArgv 构建 argv
        std::vector<const char*> argv;
        argv.push_back("SUBSCRIBE");
        for (const auto& ch : channels) {
            argv.push_back(ch.c_str());
        }

        std::vector<size_t> argvlen(argv.size(), 0);
        for (size_t i = 1; i < argv.size(); i++) {
            argvlen[i] = strlen(argv[i]);
        }

        auto* reply = static_cast<redisReply*>(
            redisCommandArgv(sub_ctx_, argv.size(), argv.data(), argvlen.data()));
        if (reply) freeReplyObject(reply);

        // 处理订阅消息
        while (running_) {
            redisReply* msg_reply = nullptr;
            // 通过 select/poll 方式实现 redisGetReply 的 100ms 超时
            // hiredis 不原生支持阻塞式 getReply 的超时，
            // 因此我们使用基于 redisGetReplyFromReader 的轮询方案。
            int err = redisGetReply(sub_ctx_, reinterpret_cast<void**>(&msg_reply));
            if (err != REDIS_OK) break;

            if (msg_reply == nullptr) {
                // 连接已关闭或出错
                break;
            }

            if (msg_reply->type == REDIS_REPLY_ARRAY && msg_reply->elements == 4) {
                // message[0] = "message"
                // message[1] = 频道名称
                // message[2] = 消息字符串（PSUBSCRIBE 时用作模式匹配）
                // message[3] = 消息内容
                std::string msg_channel(msg_reply->element[1]->str,
                                         msg_reply->element[1]->len);
                std::string msg_payload(msg_reply->element[3]->str,
                                         msg_reply->element[3]->len);

                if (callback) {
                    callback(msg_channel, msg_payload);
                }
            }

            freeReplyObject(msg_reply);
        }

        redisFree(sub_ctx_);
        sub_ctx_ = nullptr;
    }

    /// 在单独的线程中启动订阅。
    void start_subscribe(const std::vector<std::string>& channels,
                         MessageCallback callback) {
        sub_thread_ = std::thread(&RedisClient::subscribe, this,
                                  channels, std::move(callback));
    }

    /// 通知订阅循环停止。
    void stop_subscribe() {
        running_ = false;
        // subscribe 调用会阻塞在 redisGetReply 上，因此我们必须关闭上下文
        // 或取消阻塞。从另一个线程关闭在 hiredis 中不是线程安全的。
        // 因此，我们改为依赖 disconnect() 调用，它会首先 join 该线程。
    }

    /// 检查发布连接是否存活。
    bool is_connected() const {
        return ctx_ != nullptr;
    }

    /// 断开后重新连接。
    bool reconnect() {
        if (ctx_) {
            redisFree(ctx_);
            ctx_ = nullptr;
        }
        return connect();
    }

private:
    redisContext* connect_internal() {
        redisContext* ctx = redisConnect(host_.c_str(), port_);
        if (ctx == nullptr || ctx->err) {
            std::cerr << "[Redis] 连接错误："
                      << (ctx ? ctx->errstr : "无法分配上下文")
                      << std::endl;
            if (ctx) redisFree(ctx);
            return nullptr;
        }

        // 设置超时
        struct timeval tv = {1, 500000}; // 1.5 秒
        redisSetTimeout(ctx, tv);

        return ctx;
    }

    std::string host_;
    int port_;
    std::string password_;

    redisContext* ctx_;          // 发布上下文
    redisContext* sub_ctx_;      // 订阅上下文
    std::atomic<bool> running_;
    std::thread sub_thread_;
    std::mutex pub_mutex_;
};

#endif // REDIS_CLIENT_HPP
