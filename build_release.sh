#!/bin/bash

# =============================================================================
# Rakuraku Music Station - Build Script v3.0
# =============================================================================
#
# Usage:
#   ./build_release.sh              # Full build (Rust backend + engine)
#
# =============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status()  { echo -e "${BLUE}[*]${NC} $1"; }
print_success() { echo -e "${GREEN}[✓]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[!]${NC} $1"; }
print_error()   { echo -e "${RED}[✗]${NC} $1"; }

echo -e "${BLUE}
══════════════════════════════════════════════
    Rakuraku Music Station v3.0 构建工具
══════════════════════════════════════════════${NC}
"

print_status "检测系统环境..."
if [ -f /etc/arch-release ]; then
    DEPENDENCIES="ffmpeg"
    INSTALL_CMD="sudo pacman -S --needed --noconfirm"
elif [ -f /etc/debian_version ]; then
    DEPENDENCIES="ffmpeg"
    INSTALL_CMD="sudo apt-get install -y"
else
    print_warning "未识别的 Linux 发行版，请手动安装依赖: ffmpeg"
    DEPENDENCIES=""
    INSTALL_CMD=""
fi

# 检查运行时依赖
for dep in $DEPENDENCIES; do
    if command -v "$dep" &>/dev/null; then
        print_success "找到 $dep"
    else
        print_warning "$dep 未找到，尝试安装..."
        $INSTALL_CMD $dep
    fi
done

# 检查 Rust 工具链
if command -v cargo &>/dev/null; then
    print_success "Rust 工具链: $(rustc --version)"
else
    print_error "未找到 Rust 工具链，请先安装: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 构建 Rust 后端 + 音频引擎
print_status "构建 Rust 后端（含音频引擎）..."
cd radio-backend

print_status "编译 release 构建..."
cargo build --release 2>&1
print_success "Rust release 构建完成"

cd ..

# 创建 dist 目录
print_status "准备部署文件..."
mkdir -p dist/data dist/media

# 复制二进制文件
cp radio-backend/target/release/radio-backend dist/
print_success "radio-backend 二进制文件已复制"

# 复制前端静态文件（如果存在）
if [ -d "radio-backend/static" ]; then
    rm -r dist/static
    cp -r radio-backend/static dist/
    print_success "前端静态文件已复制"
fi

# 复制默认配置（如果尚未存在）
if [ ! -f "dist/config.toml" ]; then
    cp radio-backend/config.toml.example dist/config.toml
    print_success "默认 config.toml 已复制"
fi

# 保留现存的 media/ 和 playlist 数据
print_success "dist/ 目录准备完毕（media/ 和 playlist 数据已保留）"

# 生成启动脚本
cat > dist/start.sh << 'STARTEMBED'
#!/bin/bash
cd "$(dirname "$0")"

echo "启动 Rakuraku Music Station v3.0..."

if [ -f .server.pid ] && kill -0 $(cat .server.pid) 2>/dev/null; then
    echo "服务器已在运行中 (PID $(cat .server.pid))"
    exit 1
fi

nohup ./radio-backend > server.log 2>&1 &
PID=$!
echo $PID > .server.pid
echo "服务器已启动 (PID $PID)"
echo "日志文件: server.log"
echo "访问 http://localhost:2241"
STARTEMBED
chmod +x dist/start.sh

cat > dist/stop.sh << 'STOPEMBED'
#!/bin/bash
cd "$(dirname "$0")"

echo "停止 Rakuraku Music Station v3.0..."

if [ -f .server.pid ]; then
    PID=$(cat .server.pid)
    if kill -0 $PID 2>/dev/null; then
        kill $PID
        echo "服务器已停止 (PID $PID)"
        rm -f .server.pid
    else
        echo "PID $PID 对应的进程已不存在"
        rm -f .server.pid
    fi
else
    echo "未找到 PID 文件"
fi

# 额外清理残留进程
pkill -f "radio-backend" 2>/dev/null && echo "清理了残留的 radio-backend 进程" || true
STOPEMBED
chmod +x dist/stop.sh

print_success "启动/停止脚本已生成"

echo ""
echo -e "${GREEN}══════════════════════════════════════════════${NC}"
echo -e "${GREEN}  构建完成！${NC}"
echo ""
echo "  使用方法:"
echo "    cd dist"
echo "    ./start.sh   # 启动服务"
echo "    ./stop.sh    # 停止服务"
echo ""
echo "  首次使用:"
echo "    1. 编辑 dist/config.toml 设置 admin_setup_token"
echo "    2. 将音频文件放入 dist/media/"
echo "    3. ./start.sh"
echo "    4. 访问 http://localhost:2241"
echo "    5. 点击导航栏 🔑 按钮，输入 admin_setup_token 获取管理员权限"
echo "    6. 在管理后台配置网易云 Cookie (MUSIC_U) 以启用批量下载"
echo -e "${GREEN}══════════════════════════════════════════════${NC}"
