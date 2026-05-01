/// 用于 "push-file" 播放模式的 HTTP Range 文件服务器。
/// 处理 `/file/{song_id}` 端点，通过 HTTP Range 提供音频文件分块。
/// 与现有的 Crow HTTP 框架集成。

#ifndef FILE_SERVER_HPP
#define FILE_SERVER_HPP

#include <string>
#include <fstream>
#include <sstream>
#include <filesystem>
#include <sys/stat.h>
#include <sys/sendfile.h>
#include <fcntl.h>
#include <unistd.h>

namespace fs = std::filesystem;

/// 解析 HTTP Range 头部的值。
/// 返回 {start, end} 偏移量。-1 表示开放式范围。
struct RangeRequest {
    int64_t start = 0;
    int64_t end = -1;   // -1 = 到文件末尾
    bool valid = false;

    static RangeRequest parse(const std::string& header, int64_t file_size) {
        RangeRequest req;
        if (header.empty() || header.find("bytes=") != 0) {
            req.valid = false;
            return req;
        }

        std::string range_spec = header.substr(6); // 去除 "bytes=" 前缀
        auto dash = range_spec.find('-');
        if (dash == std::string::npos) {
            req.valid = false;
            return req;
        }

        std::string start_str = range_spec.substr(0, dash);
        std::string end_str = range_spec.substr(dash + 1);

        if (start_str.empty()) {
            // 后缀范围："bytes=-500" 表示最后 500 字节
            try {
                int64_t suffix = std::stoll(end_str);
                req.start = std::max(int64_t(0), file_size - suffix);
                req.end = file_size - 1;
                req.valid = true;
            } catch (...) {
                req.valid = false;
            }
        } else {
            try {
                req.start = std::stoll(start_str);
                if (end_str.empty()) {
                    req.end = file_size - 1; // 开放式（到文件末尾）
                } else {
                    req.end = std::stoll(end_str);
                }
                req.valid = (req.start >= 0 && req.start < file_size && req.end >= req.start);
            } catch (...) {
                req.valid = false;
            }
        }

        return req;
    }
};

/// 生成文件响应的头部。
/// 如果 range 有效，返回 206 Partial Content 及 Content-Range。
/// 否则返回 200 及完整的 Content-Length。
std::string build_file_response_headers(const std::string& mime_type,
                                        int64_t file_size,
                                        const RangeRequest& range,
                                        bool& is_partial) {
    std::ostringstream headers;

    if (range.valid) {
        is_partial = true;
        int64_t content_length = range.end - range.start + 1;
        headers << "HTTP/1.1 206 Partial Content\r\n";
        headers << "Content-Range: bytes " << range.start << "-" << range.end
                << "/" << file_size << "\r\n";
        headers << "Content-Length: " << content_length << "\r\n";
    } else {
        is_partial = false;
        headers << "HTTP/1.1 200 OK\r\n";
        headers << "Content-Length: " << file_size << "\r\n";
    }

    headers << "Content-Type: " << mime_type << "\r\n";
    headers << "Accept-Ranges: bytes\r\n";
    headers << "Cache-Control: public, max-age=3600\r\n";
    headers << "Access-Control-Allow-Origin: *\r\n";
    headers << "Connection: keep-alive\r\n";
    headers << "Server: Rakuraku-Radio-File\r\n";
    headers << "\r\n";

    return headers.str();
}

/// 根据文件扩展名获取 MIME 类型。
std::string get_mime_type(const std::string& filename) {
    auto dot = filename.find_last_of('.');
    if (dot == std::string::npos) return "application/octet-stream";

    std::string ext = filename.substr(dot);
    if (ext == ".mp3") return "audio/mpeg";
    if (ext == ".wav") return "audio/wav";
    if (ext == ".flac") return "audio/flac";
    if (ext == ".ogg" || ext == ".oga") return "audio/ogg";
    if (ext == ".m4a") return "audio/mp4";
    if (ext == ".aac") return "audio/aac";
    if (ext == ".opus") return "audio/opus";
    if (ext == ".lrc") return "text/plain; charset=utf-8";
    if (ext == ".jpg" || ext == ".jpeg") return "image/jpeg";
    if (ext == ".png") return "image/png";

    return "application/octet-stream";
}

/// 提供带 Range 支持的文件服务。直接写入给定的 fd。
/// 返回 true 表示文件已成功提供（部分或全部），false 表示出错。
bool serve_file_range(int client_fd, const std::string& file_path,
                      const std::string& range_header) {
    // 打开文件
    int fd = open(file_path.c_str(), O_RDONLY);
    if (fd < 0) {
        std::string err = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
        ::send(client_fd, err.c_str(), err.size(), MSG_NOSIGNAL);
        return false;
    }

    // 获取文件大小
    struct stat st;
    if (fstat(fd, &st) < 0) {
        close(fd);
        return false;
    }
    int64_t file_size = st.st_size;

    // 解析 Range 头部
    RangeRequest range = RangeRequest::parse(range_header, file_size);
    std::string mime = get_mime_type(file_path);
    bool is_partial;

    // 发送头部
    std::string headers = build_file_response_headers(mime, file_size, range, is_partial);
    ::send(client_fd, headers.c_str(), headers.size(), MSG_NOSIGNAL);

    // 发送文件数据
    off_t offset = range.valid ? static_cast<off_t>(range.start) : 0;
    size_t count = range.valid
        ? static_cast<size_t>(range.end - range.start + 1)
        : static_cast<size_t>(file_size);

    // 使用 sendfile 实现零拷贝传输
    ssize_t remaining = static_cast<ssize_t>(count);
    while (remaining > 0) {
        ssize_t sent = sendfile(client_fd, fd, &offset, remaining);
        if (sent <= 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                // 短暂暂停后重试
                usleep(1000);
                continue;
            }
            break;
        }
        remaining -= sent;
    }

    close(fd);
    return true;
}

#endif // FILE_SERVER_HPP
