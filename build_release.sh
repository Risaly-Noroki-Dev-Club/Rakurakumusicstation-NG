#!/bin/bash

# =============================================================================
# Rakuraku Music Station - Build Script v2.1
# =============================================================================
#
# Usage:
#   ./build_release.sh              # Full build (C++ + Rust, auto-download crow_all.h)
#   ./build_release.sh --no-crow    # Skip crow_all.h download (fail if missing)
#   ./build_release.sh --skip-rust  # Skip Rust build
#
# =============================================================================

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status()  { echo -e "${BLUE}[*]${NC} $1"; }
print_success() { echo -e "${GREEN}[✓]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[!]${NC} $1"; }
print_error()   { echo -e "${RED}[✗]${NC} $1"; }

# 解析参数
AUTO_CROW=true
SKIP_RUST=false
for arg in "$@"; do
    case "$arg" in
        --no-crow)  AUTO_CROW=false ;;
        --skip-rust) SKIP_RUST=true ;;
    esac
done

echo -e "${BLUE}
══════════════════════════════════════════════
    Rakuraku Music Station 构建工具
══════════════════════════════════════════════${NC}
"

# 检查系统环境
print_status "检测系统环境..."
if [ -f /etc/arch-release ]; then
    OS="Arch Linux"
    PKG_MGR="pacman"
    INSTALL_CMD="sudo pacman -S --needed --noconfirm"
    DEPENDENCIES="base-devel ffmpeg openssl wget asio"
elif [ -f /etc/debian_version ] || [ -f /etc/lsb-release ]; then
    OS="Debian/Ubuntu"
    PKG_MGR="apt"
    INSTALL_CMD="sudo apt-get install -y"
    DEPENDENCIES="build-essential ffmpeg libssl-dev wget libasio-dev"
else
    print_error "不支持的操作系统。仅支持 Arch Linux 和 Debian/Ubuntu 系列"
    exit 1
fi

print_success "检测到系统: $OS"

# 检查并安装依赖
print_status "检查系统依赖..."
if [ "$PKG_MGR" == "apt" ]; then
    sudo apt-get update > /dev/null 2>&1
fi

# 安装系统依赖
print_status "安装系统依赖包..."
$INSTALL_CMD $DEPENDENCIES > /dev/null 2>&1

# 验证 FFmpeg
print_status "检查 FFmpeg 支持..."
if command -v ffmpeg > /dev/null 2>&1; then
    if ffmpeg -encoders | grep -q "libmp3lame"; then
        print_success "FFmpeg 支持 MP3 编码"
    else
        print_warning "FFmpeg 缺少 MP3 编码支持，将影响音频转码"
    fi
else
    print_error "FFmpeg 未找到，请确保已正确安装"
    exit 1
fi

# 检查 crow_all.h（C++ 构建必需）
print_status "检查 Crow 框架头文件..."
if [ ! -f "crow_all.h" ]; then
    if [ "$AUTO_CROW" = true ]; then
        print_warning "crow_all.h 未找到，正在自动下载..."
        if command -v wget > /dev/null 2>&1 && command -v python3 > /dev/null 2>&1; then
            CROW_URL=$(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest 2>/dev/null \
                | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))" 2>/dev/null)
            if [ -n "$CROW_URL" ]; then
                wget -q "$CROW_URL" -O crow_all.h && print_success "crow_all.h 下载完成" || {
                    print_error "下载 crow_all.h 失败"
                    print_error "请手动下载：wget <url> -O crow_all.h"
                    print_error "或参考 README.md 中的下载命令"
                    exit 1
                }
            else
                print_error "无法获取 crow_all.h 下载地址"
                print_error "请参考 README.md 手动下载"
                exit 1
            fi
        else
            print_error "缺少 wget 或 python3，无法自动下载 crow_all.h"
            print_error "请手动下载后放置到仓库根目录"
            exit 1
        fi
    else
        print_error "crow_all.h 未找到！"
        print_error "请下载后放置到仓库根目录："
        print_error "  wget \$(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest | python3 -c \"import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))\") -O crow_all.h"
        print_error "或使用 ./build_release.sh（不加 --no-crow）自动下载"
        exit 1
    fi
else
    print_success "crow_all.h 已就绪"
fi

# 编译项目
print_status "编译服务器程序..."
RELEASE_DIR="dist"
# Remove build artifacts but preserve media/ and settings.json and playlist_order.json
if [ -d "$RELEASE_DIR" ]; then
    find "$RELEASE_DIR" -mindepth 1 -maxdepth 1 ! -name 'media' ! -name 'playlist_order.json' -exec rm -rf {} +
fi
mkdir -p $RELEASE_DIR/media
mkdir -p $RELEASE_DIR/templates

# 编译参数
CXXFLAGS="-std=c++17 -O3 -flto -march=native -I. -Isrc -w"
LDFLAGS="-lpthread -lssl -lcrypto"

# 使用 Makefile 编译所有模块
make CXXFLAGS="$CXXFLAGS" LDFLAGS="$LDFLAGS" -j$(nproc)
if [ ! -f "radioserver" ]; then
    print_error "编译失败"
    exit 1
fi
cp radioserver $RELEASE_DIR/radioserver

if [ -f "$RELEASE_DIR/radioserver" ]; then
    # 可选：移除调试符号减小体积
    if command -v strip > /dev/null 2>&1; then
        strip $RELEASE_DIR/radioserver
    fi
    print_success "编译成功"
else
    print_error "编译失败"
    exit 1
fi

# 复制配置文件和工具
print_status "准备运行环境..."

# HTML 模板已内嵌进二进制，此处仅在需要运行时覆盖时才复制
# [ -f "login.html" ] && cp login.html $RELEASE_DIR/
# [ -f "panel.html" ] && cp panel.html $RELEASE_DIR/
# [ -f "index.html" ] && cp index.html $RELEASE_DIR/

# 批量下载工具
if [ -f "music_dl.py" ]; then
    cp music_dl.py $RELEASE_DIR/
    print_success "已打包 music_dl.py"
else
    print_warning "未找到 music_dl.py，跳过"
fi
if [ -f "requirements.txt" ]; then
    cp requirements.txt $RELEASE_DIR/
fi

if [ -f "index.html" ]; then
    true  # 已内嵌，无需复制
else
    # 创建基础模板
    cat > $RELEASE_DIR/index.html << 'EOF'
<!DOCTYPE html>
<html>
<head>
    <title>Rakuraku Music Station</title>
    <meta charset="UTF-8">
    <style>
        body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        .player { background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0; }
        button { padding: 10px 20px; margin: 5px; background: #764ba2; color: white; border: none; border-radius: 4px; cursor: pointer; }
    </style>
</head>
<body>
    <h1>🎵 Rakuraku Music Station</h1>
    <div class="player">
        <p>当前正在播放的音乐电台</p>
        <audio id="audioPlayer" controls style="width: 100%;">
            <source src="/stream" type="audio/mpeg">
        </audio>
        <div>
            <button onclick="document.getElementById('audioPlayer').play()">播放</button>
            <button onclick="document.getElementById('audioPlayer').pause()">暂停</button>
            <button onclick="window.location.href='/admin'">管理面板</button>
        </div>
    </div>
    <p>将音频文件放入 media/ 目录即可自动播放</p>
</body>
</html>
EOF
fi

# 创建启动脚本
cat > $RELEASE_DIR/start.sh << 'EOF'
#!/bin/bash

# 设置中文环境支持
if locale -a | grep -qi "zh_CN.utf8"; then
    export LANG=zh_CN.UTF-8
    export LC_ALL=zh_CN.UTF-8
fi

# 确保媒体目录存在
mkdir -p media

echo "🎵 Rakuraku Music Station 启动中..."
echo "========================================"
echo "C++ 音频引擎: http://localhost:2240"
echo "Rust 后端服务: http://localhost:2241"
echo "流媒体:     http://localhost:2240/stream"
echo "状态查询:   http://localhost:2240/state"
echo "命令接口:   http://localhost:2240/command"
echo "Web 界面:   http://localhost:2241"
echo "========================================"
echo "音乐文件请放置在 media/ 目录"
echo ""

# 启动 C++ 音频引擎
echo "🔧 启动 C++ 音频引擎 (端口 2240)..."
nohup ./radioserver > server.log 2>&1 &
echo $! > .server.pid
echo "✅ C++ 音频引擎已启动 (PID: $(cat .server.pid))"

# 等待 C++ 引擎就绪再启动 Rust 后端
sleep 1

# 启动 Rust 后端
if [ -f "./radio-backend" ]; then
    echo "🦀 启动 Rust 后端服务 (端口 2241)..."
    nohup ./radio-backend > rust-server.log 2>&1 &
    echo $! > .rust-server.pid
    echo "✅ Rust 后端服务已启动 (PID: $(cat .rust-server.pid))"
else
    echo "⚠️  未找到 radio-backend，跳过 Rust 后端启动"
    echo "   运行 ./build_release.sh 编译 Rust 后端以启用 Web 界面"
fi

echo ""
echo "📄 C++ 日志: tail -f server.log"
echo "📄 Rust 日志: tail -f rust-server.log"
echo "🛑 停止服务: ./stop.sh"
EOF

# 创建停止脚本
cat > $RELEASE_DIR/stop.sh << 'EOF'
#!/bin/bash

echo "🎵 Rakuraku Music Station 停止脚本"
echo "========================================"

# 通用停止函数：SIGTERM → 等待最多10s → SIGKILL
stop_process() {
    local name="$1"
    local pid_file="$2"

    if [ -f "$pid_file" ]; then
        PID=$(cat "$pid_file")
        if ps -p "$PID" > /dev/null 2>&1; then
            echo "🔴 正在停止 $name (PID: $PID)..."

            # 先发送SIGTERM，优雅关闭
            kill "$PID"

            # 等待进程退出，最多等待10秒
            for i in {1..10}; do
                if ! ps -p "$PID" > /dev/null 2>&1; then
                    break
                fi
                echo "⏳ 等待 $name 退出... ($i/10)"
                sleep 1
            done

            if ! ps -p "$PID" > /dev/null 2>&1; then
                rm -f "$pid_file"
                echo "✅ $name 已正常停止 (PID: $PID)"
            else
                echo "⚠️ $name 仍在运行，强制终止..."
                kill -9 "$PID"
                sleep 1
                rm -f "$pid_file"
                echo "✅ $name 已强制停止 (PID: $PID)"
            fi
        else
            rm -f "$pid_file"
            echo "⚠️ $name PID 文件存在但进程 $PID 已终止，已清理PID文件"
        fi
    else
        echo "ℹ️  未找到 $name PID 文件 ($pid_file)"
    fi
}

# 二次清理函数：pgrep/pkill 扫尾
cleanup_residual() {
    local name="$1"
    local pattern="$2"

    RESIDUAL=$(pgrep -f "$pattern" 2>/dev/null)
    if [ -n "$RESIDUAL" ]; then
        echo "🔄 发现残留的 $name 进程，正在清理..."
        echo "📋 进程ID: $RESIDUAL"

        # 先尝试优雅终止
        pkill -f "$pattern" 2>/dev/null
        sleep 2

        # 再次检查并强制终止任何残留进程
        REMAINING=$(pgrep -f "$pattern" 2>/dev/null)
        if [ -n "$REMAINING" ]; then
            echo "⚠️  仍有 $name 进程残留，强制清理中..."
            kill -9 $REMAINING 2>/dev/null
            sleep 1
        fi

        echo "✅ 已清理所有 $name 进程"
    else
        echo "ℹ️  没有发现残留的 $name 进程"
    fi
}

# 停止 C++ 音频引擎
stop_process "C++ 音频引擎" ".server.pid"

# 停止 Rust 后端
stop_process "Rust 后端服务" ".rust-server.pid"

echo ""

# 二次清理：确保没有残留进程
cleanup_residual "radioserver" "radioserver"
cleanup_residual "radio-backend" "radio-backend"

echo "========================================"
echo "🛑 停止脚本执行完成"
EOF

chmod +x $RELEASE_DIR/start.sh $RELEASE_DIR/stop.sh

# =============================================================================
# Rust 后端编译
# =============================================================================
if [ "$SKIP_RUST" = true ]; then
    print_warning "跳过 Rust 后端编译（--skip-rust）"
elif [ -d "radio-backend" ] && [ -f "radio-backend/Cargo.toml" ]; then
    if command -v cargo > /dev/null 2>&1; then
        print_status "编译 Rust 后端（radio-backend）..."

        # 检测迁移文件变更，强制清理 Cargo 缓存防止增量编译跳过 .sql 文件更新
        MIGRATION_HASH_FILE="radio-backend/.migration_hash"
        CURRENT_MIGRATION_HASH=$(find radio-backend/migrations -name '*.sql' -exec sha256sum {} \; | sort | sha256sum | awk '{print $1}' 2>/dev/null || echo "")
        STORED_MIGRATION_HASH=$(cat "$MIGRATION_HASH_FILE" 2>/dev/null || echo "")
        if [ -n "$CURRENT_MIGRATION_HASH" ] && [ "$CURRENT_MIGRATION_HASH" != "$STORED_MIGRATION_HASH" ]; then
            if [ -n "$STORED_MIGRATION_HASH" ]; then
                print_warning "迁移文件已变更，清理 Rust 编译缓存..."
            fi
            (cd radio-backend && cargo clean 2>&1 | tail -1)
        fi

        (cd radio-backend && cargo build --release 2>&1 | while IFS= read -r line; do
            case "$line" in
                *"Compiling"*|*"Finished"*) echo "  $line" ;;
            esac
        done)
        echo "$CURRENT_MIGRATION_HASH" > "$MIGRATION_HASH_FILE"
        if [ -f "radio-backend/target/release/radio-backend" ]; then
            cp radio-backend/target/release/radio-backend "$RELEASE_DIR/"
            # 复制静态资源和配置模板
            if [ -d "radio-backend/static" ]; then
                cp -r radio-backend/static "$RELEASE_DIR/"
            fi
            if [ -f "radio-backend/config.toml.example" ]; then
                if [ ! -f "$RELEASE_DIR/config.toml" ]; then
                    cp radio-backend/config.toml.example "$RELEASE_DIR/config.toml"
                    print_warning "已生成 config.toml，请检查并修改配置"
                else
                    print_success "保留现有 config.toml"
                fi
            fi
            print_success "Rust 后端编译完成"
        else
            print_warning "Rust 编译未生成二进制，请检查 radio-backend/"
        fi
    else
        print_warning "未检测到 Rust 工具链（cargo），跳过 Rust 后端编译"
        print_warning "安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
else
    print_warning "未找到 radio-backend/Cargo.toml，跳过 Rust 后端编译"
fi

# 完成提示
print_success "构建完成！"
echo -e "${BLUE}
使用方法:
══════════════════════════════════════════════${NC}"
echo "1. 进入目录: cd $RELEASE_DIR"
echo "2. 添加音乐: 将音频文件放入 media/ 目录"
echo "3. 启动服务: ./start.sh（同时启动 C++ 引擎和 Rust 后端）"
echo "4. 音频流: http://localhost:2240/stream"
echo "5. Web 界面: http://localhost:2241"
echo ""
echo "支持格式: MP3, WAV, FLAC, OGG, M4A, AAC"
echo ""
if [ "$SKIP_RUST" = false ] && [ -f "$RELEASE_DIR/radio-backend" ]; then
    echo -e "${BLUE}Rust 后端已编译，./start.sh 将同时启动两个服务${NC}"
else
    echo "如需 Web/API 功能，安装 cargo 后重新运行: ./build_release.sh"
fi
echo ""

# ── 流地址配置 ──────────────────────────────────────────
if [ -f "$RELEASE_DIR/config.toml" ]; then
    CURRENT_STREAM=$(grep 'stream_base' "$RELEASE_DIR/config.toml" 2>/dev/null | head -1)
    if [ -z "$CURRENT_STREAM" ] || echo "$CURRENT_STREAM" | grep -q 'stream_base = "/stream"'; then
        echo -e "${BLUE}══════════════════════════════════════════════
  音频流地址配置
══════════════════════════════════════════════${NC}"
        echo ""
        echo "音频流地址决定了浏览器如何加载电台音频。"
        echo "  [1] 相对路径 /stream  — 适用于同机访问或反向代理"
        echo "  [2] 绝对地址          — 适用于外网直接访问（需指定 IP/域名）"
        echo ""
        read -p "请选择 [1/2] (默认 1): " STREAM_CHOICE
        STREAM_CHOICE=${STREAM_CHOICE:-1}

        if [ "$STREAM_CHOICE" = "2" ]; then
            read -p "请输入音频流完整地址 (例如 http://192.168.1.100:2240/stream): " ABS_URL
            if [ -n "$ABS_URL" ]; then
                if command -v python3 > /dev/null 2>&1; then
                    python3 -c "
import sys
path = '$RELEASE_DIR/config.toml'
content = open(path).read()
if \"stream_base\" in content:
    import re
    content = re.sub(r'stream_base\s*=\s*\"[^\"]*\"', 'stream_base = \"$ABS_URL\"', content)
else:
    content = content.replace('[audio_engine]', '[audio_engine]\nstream_base = \"$ABS_URL\"')
open(path, 'w').write(content)
" 2>/dev/null
                    print_success "流地址已设置为: $ABS_URL"
                else
                    sed -i "s|stream_base = \"/stream\"|stream_base = \"$ABS_URL\"|" "$RELEASE_DIR/config.toml" 2>/dev/null
                    if grep -q 'stream_base' "$RELEASE_DIR/config.toml"; then
                        print_success "流地址已设置为: $ABS_URL"
                    else
                        print_warning "自动配置失败，请手动编辑 config.toml 的 stream_base"
                    fi
                fi
            fi
        else
            print_success "使用默认相对路径: /stream"
        fi
        echo ""
    fi
fi

echo -e "${GREEN}🎵 享受音乐时光！${NC}"
