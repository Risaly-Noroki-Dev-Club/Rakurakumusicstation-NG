#ifndef SESSION_MANAGER_HPP
#define SESSION_MANAGER_HPP

#include <string>
#include <unordered_map>
#include <chrono>
#include <mutex>
#include <memory>
#include <iostream>
#include <stdexcept>
#include <openssl/crypto.h>
#include <openssl/rand.h>

class SessionManager {
public:
    struct Session {
        std::string session_id;
        std::string user_id;
        std::chrono::system_clock::time_point created_at;
        std::chrono::system_clock::time_point last_accessed;
        bool is_admin;
        
        Session(const std::string& user_id, bool admin = false)
            : session_id(generate_session_id()), user_id(user_id), 
              created_at(std::chrono::system_clock::now()),
              last_accessed(std::chrono::system_clock::now()),
              is_admin(admin) {}
        
        bool is_expired(int expiry_hours = 24) const {
            auto now = std::chrono::system_clock::now();
            auto duration = std::chrono::duration_cast<std::chrono::hours>(now - last_accessed);
            return duration.count() > expiry_hours;
        }
        
        void update_access_time() {
            last_accessed = std::chrono::system_clock::now();
        }
        
        static std::string generate_session_id() {
            unsigned char buf[16];  // 128 bits
            if (RAND_bytes(buf, sizeof(buf)) != 1) {
                throw std::runtime_error("RAND_bytes failed");
            }
            static const char* hex_chars = "0123456789abcdef";
            std::string session_id;
            session_id.reserve(32);
            for (unsigned char b : buf) {
                session_id += hex_chars[b >> 4];
                session_id += hex_chars[b & 0xf];
            }
            return session_id;
        }
    };
    
    // 创建管理员会话
    std::shared_ptr<Session> create_admin_session(const std::string& user_id = "admin") {
        std::lock_guard<std::mutex> lock(mutex_);
        auto session = std::make_shared<Session>(user_id, true);
        sessions_[session->session_id] = session;
        
        // 清理过期会话
        cleanup_expired_sessions();
        
        return session;
    }
    
    // 验证会话
    std::shared_ptr<Session> get_session(const std::string& session_id) {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = sessions_.find(session_id);
        if (it != sessions_.end()) {
            if (it->second->is_expired()) {
                sessions_.erase(it);
                return nullptr;
            }
            it->second->update_access_time();
            return it->second;
        }
        return nullptr;
    }
    
    // 销毁会话
    void destroy_session(const std::string& session_id) {
        std::lock_guard<std::mutex> lock(mutex_);
        sessions_.erase(session_id);
    }
    
    // 清理所有过期会话
    void cleanup_expired_sessions() {
        auto now = std::chrono::system_clock::now();
        auto it = sessions_.begin();
        while (it != sessions_.end()) {
            if (it->second->is_expired()) {
                it = sessions_.erase(it);
            } else {
                ++it;
            }
        }
    }
    
    // 常数时间字符串比较（防止时序攻击）
    static bool constant_time_str_eq(const std::string& a, const std::string& b) {
        if (a.size() != b.size()) return false;
        return CRYPTO_memcmp(a.data(), b.data(), a.size()) == 0;
    }

    // 密码验证
    bool verify_password(const std::string& password, const std::string& admin_password) {
        return constant_time_str_eq(password, admin_password);
    }
    
private:
    std::mutex mutex_;
    std::unordered_map<std::string, std::shared_ptr<Session>> sessions_;
    std::unordered_map<std::string, std::string> user_passwords_;
};

#endif // SESSION_MANAGER_HPP
