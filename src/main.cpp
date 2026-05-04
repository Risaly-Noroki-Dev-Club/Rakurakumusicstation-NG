#include "radio_server.hpp"

#include <csignal>
#include <iostream>

RadioServer* g_server_instance = nullptr;

static void signal_handler(int) {
    if (g_server_instance) g_server_instance->request_stop();
}

int main() {
    std::signal(SIGINT, signal_handler);
    std::signal(SIGTERM, signal_handler);
    std::signal(SIGPIPE, SIG_IGN);

    try {
        RadioServer server;
        g_server_instance = &server;

        if (!server.start()) {
            std::cerr << "[System] 引擎启动失败" << std::endl;
            return 1;
        }

        server.wait_for_shutdown();
        std::cout << "\n[System] 收到终止信号，正在关闭引擎..." << std::endl;
        server.stop();
        g_server_instance = nullptr;

    } catch (const std::exception& e) {
        std::cerr << "[System] 致命错误: " << e.what() << std::endl;
        return 1;
    }

    std::cout << "[System] 引擎已退出" << std::endl;
    return 0;
}
