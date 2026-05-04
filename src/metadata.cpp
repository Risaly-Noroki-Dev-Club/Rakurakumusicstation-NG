#include "metadata.hpp"
#include <iostream>
#include <algorithm>
#include <cstring>
#include <filesystem>
#include <regex>
#include <sstream>
#include <cctype>
#include <unistd.h>
#include <fcntl.h>
#include <sys/wait.h>

using namespace std;
namespace fs = std::filesystem;

// TrackMetadata 实现

TrackMetadata::TrackMetadata(const std::string& file_path) : file_path(file_path) {
    filename = file_path.substr(file_path.find_last_of("/\\") + 1);

    // 尝试从文件名推测基本信息
    if (title.empty()) {
        title = filename.substr(0, filename.find_last_of('.'));
    }
}

bool TrackMetadata::is_empty() const {
    return title.empty() && artist.empty() && album.empty();
}

std::string TrackMetadata::get_display_name() const {
    if (!title.empty()) {
        return title;
    }
    return filename.substr(0, filename.find_last_of('.'));
}

void TrackMetadata::clear() {
    filename.clear();
    title.clear();
    artist.clear();
    album.clear();
    genre.clear();
    year = 0;
    track_number = 0;
    duration = 0;
    cover_art.clear();
    lyrics.clear();
    file_path.clear();
}

// MetadataManager 实现

TrackMetadata MetadataManager::extract_metadata(const std::string& file_path) {
    TrackMetadata metadata;
    metadata.file_path = file_path;
    metadata.filename = file_path.substr(file_path.find_last_of("/\\") + 1);

    if (!fs::exists(file_path)) {
        cerr << "[Metadata] 文件不存在: " << file_path << endl;
        return metadata;
    }

    // 首先尝试从文件名中提取基本信息
    string name_without_ext = metadata.filename.substr(0, metadata.filename.find_last_of('.'));

    // 常见音乐文件命名模式：艺术家 - 歌曲名
    regex pattern1("(.*)\\s*-\\s*(.*)");
    smatch matches;

    if (regex_match(name_without_ext, matches, pattern1) && matches.size() == 3) {
        metadata.artist = matches[1].str();
        metadata.title = matches[2].str();
        cout << "[Metadata] 从文件名提取: " << metadata.artist << " - " << metadata.title << endl;
    } else {
        // 如果没有匹配到常见模式，直接使用文件名作为标题
        metadata.title = name_without_ext;
    }

    // 使用FFmpeg获取时长信息
    metadata.duration = get_duration_via_ffmpeg(file_path);

    // 从内嵌元数据提取歌词
    metadata.lyrics = get_lyrics_via_ffprobe(file_path);

    cout << "[Metadata] 成功提取基础元数据: " << metadata.get_display_name()
         << " (" << metadata.duration << "秒)" << endl;

    return metadata;
}

bool MetadataManager::is_supported_format(const std::string& filename) {
    vector<string> supported_formats = {".mp3", ".flac", ".ogg", ".wav", ".m4a", ".aac"};
    string extension = fs::path(filename).extension().string();
    transform(extension.begin(), extension.end(), extension.begin(), ::tolower);

    for (const auto& fmt : supported_formats) {
        if (extension == fmt) {
            return true;
        }
    }
    return false;
}

std::string MetadataManager::safe_filename(const std::string& filename) {
    // 简单的文件名清理，移除可能的问题字符
    string safe_name = filename;
    string invalid_chars = "\\/:*?\"<>|";

    for (char c : invalid_chars) {
        replace(safe_name.begin(), safe_name.end(), c, '_');
    }

    return safe_name;
}

int MetadataManager::get_duration_via_ffmpeg(const std::string& file_path) {
    // 不通过 shell 调用 ffmpeg，避免文件名中的特殊字符触发命令注入。
    // ffmpeg 把 Duration 信息写到 stderr，因此把子进程的 stderr 接到管道。
    int fds[2];
    if (pipe2(fds, O_CLOEXEC) != 0) {
        cerr << "[Metadata] pipe2 失败: " << strerror(errno) << endl;
        return 0;
    }

    pid_t pid = fork();
    if (pid < 0) {
        cerr << "[Metadata] fork 失败: " << strerror(errno) << endl;
        close(fds[0]);
        close(fds[1]);
        return 0;
    }

    if (pid == 0) {
        // 子进程：把 stderr 接到管道写端，然后 exec ffmpeg。
        if (dup2(fds[1], STDERR_FILENO) < 0) _exit(127);
        // stdout 重定向到 /dev/null，避免污染日志/管道
        int devnull = open("/dev/null", O_WRONLY);
        if (devnull >= 0) {
            dup2(devnull, STDOUT_FILENO);
            close(devnull);
        }
        // O_CLOEXEC 让 fds[0]/fds[1] 在 exec 后自动关闭
        const char* argv[] = {
            "ffmpeg", "-nostdin", "-i", file_path.c_str(), nullptr
        };
        execvp("ffmpeg", const_cast<char* const*>(argv));
        _exit(127);
    }

    // 父进程：只读管道。
    close(fds[1]);

    string result;
    char buffer[256];
    ssize_t n;
    while ((n = read(fds[0], buffer, sizeof(buffer))) > 0) {
        result.append(buffer, buffer + n);
        if (result.size() > 64 * 1024) break;  // 防止极端输入撑爆内存
    }
    close(fds[0]);

    int status = 0;
    while (waitpid(pid, &status, 0) < 0) {
        if (errno != EINTR) break;
    }

    // 解析FFmpeg输出的时长信息
    regex duration_regex("Duration: ([0-9]+):([0-9]+):([0-9]+)\\.([0-9]+)");
    smatch matches;

    if (regex_search(result, matches, duration_regex) && matches.size() >= 4) {
        try {
            int hours = stoi(matches[1].str());
            int minutes = stoi(matches[2].str());
            int seconds = stoi(matches[3].str());

            return hours * 3600 + minutes * 60 + seconds;
        } catch (const exception& e) {
            cerr << "[Metadata] 解析时长失败: " << e.what() << endl;
        }
    }

    // 如果无法解析时长，检查文件大小估算（粗略估计）
    try {
        uintmax_t file_size = fs::file_size(file_path);
        // 简化的时长估算：对于MP3文件，大约 1MB ≈ 1分钟
        if (file_path.find(".mp3") != string::npos) {
            return min(3600, static_cast<int>(file_size / 1024 / 1024)); // 最大1小时
        }
    } catch (const exception& e) {
        cerr << "[Metadata] 无法获取文件大小: " << e.what() << endl;
    }

    return 0;
}

std::string MetadataManager::get_lyrics_via_ffprobe(const std::string& file_path) {
    int fds[2];
    if (pipe2(fds, O_CLOEXEC) != 0) {
        return "";
    }

    pid_t pid = fork();
    if (pid < 0) {
        close(fds[0]);
        close(fds[1]);
        return "";
    }

    if (pid == 0) {
        if (dup2(fds[1], STDOUT_FILENO) < 0) _exit(127);
        int devnull = open("/dev/null", O_WRONLY);
        if (devnull >= 0) {
            dup2(devnull, STDERR_FILENO);
            close(devnull);
        }
        const char* argv[] = {
            "ffprobe", "-v", "error",
            "-show_entries", "format_tags",
            "-of", "default",
            file_path.c_str(), nullptr
        };
        execvp("ffprobe", const_cast<char* const*>(argv));
        _exit(127);
    }

    close(fds[1]);

    std::string output;
    char buffer[256];
    ssize_t n;
    while ((n = read(fds[0], buffer, sizeof(buffer))) > 0) {
        output.append(buffer, buffer + n);
        if (output.size() > 256 * 1024) break;
    }
    close(fds[0]);

    int status = 0;
    while (waitpid(pid, &status, 0) < 0) {
        if (errno != EINTR) break;
    }

    // 解析 ffprobe 输出的 format_tags，查找歌词标签
    // 输出格式: TAG:<key>=<value>
    std::istringstream stream(output);
    std::string line;
    while (std::getline(stream, line)) {
        // 跳过非 TAG 行（如 [FORMAT] / [/FORMAT]）
        if (line.find("TAG:") != 0) continue;

        std::string tag = line.substr(4);  // skip "TAG:"
        size_t eq = tag.find('=');
        if (eq == std::string::npos) continue;

        std::string key = tag.substr(0, eq);
        std::string value = tag.substr(eq + 1);

        // 将 key 转小写后匹配
        std::string key_lower = key;
        for (char& c : key_lower) c = static_cast<char>(std::tolower(c));

        if (key_lower == "lyrics" || key_lower == "unsyncedlyrics" ||
            key_lower == "lyrics-eng" || key_lower == "lyrics-chi" ||
            key_lower == "syncedlyrics" || key_lower.find("lyrics") != std::string::npos) {
            return value;
        }
    }

    return "";
}