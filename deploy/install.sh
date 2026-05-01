#!/bin/bash
# Rakuraku Music Station NG - 一键安装脚本
# 用法: sudo bash install.sh
# =============================================================================

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[信息]${NC} $1"; }
warn()  { echo -e "${YELLOW}[警告]${NC} $1"; }
err()   { echo -e "${RED}[错误]${NC} $1"; }

# 检查 root 权限
if [[ $EUID -ne 0 ]]; then
    err "此脚本必须以 root 身份运行（sudo）"
    exit 1
fi

info "=============================================="
info " Rakuraku Music Station NG - 安装脚本"
info "=============================================="
echo ""

# ─── 系统依赖 ────────────────────────────────────────────────────
info "正在安装系统依赖..."

if command -v apt-get >/dev/null 2>&1; then
    apt-get update -qq
    apt-get install -y -qq \
        build-essential cmake \
        ffmpeg \
        libssl-dev \
        libhiredis-dev \
        redis-server \
        curl \
        pkg-config \
        sqlite3 \
        2>&1 | tail -1
elif command -v pacman >/dev/null 2>&1; then
    pacman -S --needed --noconfirm \
        base-devel cmake \
        ffmpeg \
        openssl \
        hiredis \
        redis \
        curl \
        sqlite \
        2>&1 | tail -1
else
    err "不支持的操作系统。请手动安装依赖。"
    exit 1
fi

# 安装 Rust（如果未安装）
if ! command -v cargo >/dev/null 2>&1; then
    info "正在安装 Rust 工具链..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# ─── 构建 ──────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

info "正在构建音频引擎（C++）..."
cd "$PROJECT_DIR/audio-engine"
g++ ../radioserver.cpp ../metadata.cpp -o audio_engine \
    -std=c++17 -O3 -flto -march=native \
    -lpthread -lssl -lcrypto -lhiredis -I.. -I/usr/include/hiredis \
    && info "  -> 完成" || { err "音频引擎构建失败"; exit 1; }

info "正在构建广播后端（Rust）..."
cd "$PROJECT_DIR/radio-backend"
cargo build --release && info "  -> 完成" || { err "后端构建失败"; exit 1; }

# ─── 安装 ──────────────────────────────────────────────────────────
info "正在安装二进制文件..."
cp "$PROJECT_DIR/audio-engine/audio_engine" /usr/local/bin/
cp "$PROJECT_DIR/radio-backend/target/release/radio-backend" /usr/local/bin/
chmod +x /usr/local/bin/audio_engine /usr/local/bin/radio-backend

info "正在设置目录..."
mkdir -p /etc/rakuraku
mkdir -p /var/lib/rakuraku/media
mkdir -p /var/lib/rakuraku/data
mkdir -p /var/log/rakuraku

info "正在创建 radio 用户..."
id -u radio >/dev/null 2>&1 || useradd -r -s /bin/false -d /var/lib/rakuraku radio
chown -R radio:radio /var/lib/rakuraku /var/log/rakuraku

info "正在安装配置文件..."
[[ -f /etc/rakuraku/audio_engine.toml ]] || cp "$PROJECT_DIR/audio-engine/audio_engine.toml.example" /etc/rakuraku/audio_engine.toml
[[ -f /etc/rakuraku/radio-backend.toml ]] || cp "$PROJECT_DIR/radio-backend/config.toml.example" /etc/rakuraku/radio-backend.toml

info "正在安装 systemd 服务..."
cp "$SCRIPT_DIR/audio-engine.service" /etc/systemd/system/
cp "$SCRIPT_DIR/radio-backend.service" /etc/systemd/system/
systemctl daemon-reload

info "正在启用并启动 Redis..."
systemctl enable redis-server 2>/dev/null || systemctl enable redis 2>/dev/null || true
systemctl start redis-server 2>/dev/null || systemctl start redis 2>/dev/null || true

echo ""
info "=============================================="
info " 安装完成！"
info "=============================================="
echo ""
echo "下一步："
echo "  1. 编辑配置："
echo "     nano /etc/rakuraku/audio_engine.toml"
echo "     nano /etc/rakuraku/radio-backend.toml"
echo ""
echo "  2. 将音频文件放入 /var/lib/rakuraku/media/"
echo "     （搭配对应的 .lrc 歌词文件）"
echo ""
echo "  3. 启用服务："
echo "     systemctl enable audio-engine radio-backend"
echo ""
echo "  4. 启动服务："
echo "     systemctl start audio-engine radio-backend"
echo ""
echo "  5. 检查状态："
echo "     systemctl status audio-engine radio-backend"
echo ""
echo "  6. 访问 Web 界面："
echo "     http://$(hostname -I | awk '{print $1}'):8080"
echo "     （Rust 后端 API + WebSocket）"
echo ""
echo "  7. 音频流地址："
echo "     http://$(hostname -I | awk '{print $1}'):2240/stream"
echo ""
echo "默认管理员账户: admin / admin123"
echo "（首次使用请通过 API 或配置文件修改密码！）"
echo ""
