# Rakuraku Music Station NG - 构建与部署
# =============================================================================
#
# 目标：
#   make all              构建所有服务（默认）
#   make audio-engine     仅构建 C++ 音频引擎
#   make radio-backend    仅构建 Rust 后端
#   make clean            清除构建产物
#   make install          安装到 /usr/local/bin/ 和 /etc/rakuraku/
#   make install-systemd  安装 systemd 服务文件
#   make install-supervisor 安装 supervisor 配置
#   make uninstall        移除已安装的文件
#   make start            通过 systemd 启动服务
#   make stop             通过 systemd 停止服务
#   make status           显示服务状态
#   make client           复制客户端 HTML 到静态资源目录

.PHONY: all audio-engine radio-backend clean install install-systemd install-supervisor uninstall start stop status client

# 路径
PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
CONFDIR = /etc/rakuraku
DATADIR = /var/lib/rakuraku
SYSTEMD_DIR = /etc/systemd/system
SUPERVISOR_DIR = /etc/supervisor/conf.d
LOG_DIR = /var/log/rakuraku

# C++ 音频引擎
AUDIO_SRC = ../radioserver.cpp ../metadata.cpp
AUDIO_BIN = audio_engine
CXX = g++
CXXFLAGS = -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto -I../audio-engine -I.. -I/usr/include/hiredis

# Rust 后端
CARGO = cargo
CARGO_FLAGS = --release

# =============================================================================
# 构建
# =============================================================================

all: audio-engine radio-backend

audio-engine: $(AUDIO_SRC)
	@echo "==> 正在构建 C++ 音频引擎..."
	cd ../audio-engine && $(CXX) $(AUDIO_SRC) -o $(AUDIO_BIN) $(CXXFLAGS) -lhiredis
	@echo "    完成: audio-engine/$(AUDIO_BIN)"

radio-backend:
	@echo "==> 正在构建 Rust 后端..."
	cd ../radio-backend && $(CARGO) build $(CARGO_FLAGS)
	@echo "    完成: radio-backend/target/release/radio-backend"

clean:
	@echo "==> 正在清除构建产物..."
	rm -f ../audio-engine/audio_engine ../audio-engine/*.o ../audio-engine/*.so
	cd ../radio-backend && $(CARGO) clean
	@echo "    清除完成"

# =============================================================================
# 安装
# =============================================================================

install: install-bin install-config install-dirs
	@echo "==> 安装完成"
	@echo "    音频引擎: $(BINDIR)/audio-engine"
	@echo "    后端:      $(BINDIR)/radio-backend"
	@echo "    配置:      $(CONFDIR)/"
	@echo "    数据:      $(DATADIR)/"

install-bin:
	@echo "==> 正在安装二进制文件..."
	mkdir -p $(BINDIR)
	cp ../audio-engine/audio_engine $(BINDIR)/audio-engine 2>/dev/null || cp ../audio-engine/$(AUDIO_BIN) $(BINDIR)/audio-engine
	cp ../radio-backend/target/release/radio-backend $(BINDIR)/radio-backend 2>/dev/null || echo "    （请先构建 radio-backend：make radio-backend）"
	chmod +x $(BINDIR)/audio-engine $(BINDIR)/radio-backend

install-config:
	@echo "==> 正在安装配置文件..."
	mkdir -p $(CONFDIR)
	cp -n ../audio-engine/audio_engine.toml.example $(CONFDIR)/audio_engine.toml 2>/dev/null || true
	cp -n ../radio-backend/config.toml.example $(CONFDIR)/radio-backend.toml 2>/dev/null || true
	@echo "    配置文件已安装（已存在的文件已保留）"
	@echo "    >>> 请编辑 $(CONFDIR)/audio_engine.toml 填入你的设置"
	@echo "    >>> 请编辑 $(CONFDIR)/radio-backend.toml 填入你的设置"

install-dirs:
	@echo "==> 正在创建运行时目录..."
	mkdir -p $(DATADIR)/media
	mkdir -p $(DATADIR)/data
	mkdir -p $(LOG_DIR)
	# 如果 radio 用户不存在则创建
	id -u radio >/dev/null 2>&1 || useradd -r -s /bin/false -d $(DATADIR) radio
	chown -R radio:radio $(DATADIR) $(LOG_DIR)

install-systemd:
	@echo "==> 正在安装 systemd 服务文件..."
	cp audio-engine.service $(SYSTEMD_DIR)/
	cp radio-backend.service $(SYSTEMD_DIR)/
	systemctl daemon-reload
	@echo "    完成。启用命令：systemctl enable audio-engine radio-backend"
	@echo "    启动命令：systemctl start audio-engine radio-backend"

install-supervisor:
	@echo "==> 正在安装 supervisor 配置..."
	cp radio.conf $(SUPERVISOR_DIR)/rakuraku.conf
	supervisorctl reread
	supervisorctl update
	@echo "    完成。"

# =============================================================================
# 卸载
# =============================================================================

uninstall:
	@echo "==> 正在卸载..."
	-systemctl stop audio-engine radio-backend 2>/dev/null || true
	-systemctl disable audio-engine radio-backend 2>/dev/null || true
	rm -f $(SYSTEMD_DIR)/audio-engine.service
	rm -f $(SYSTEMD_DIR)/radio-backend.service
	rm -f $(SUPERVISOR_DIR)/rakuraku.conf
	rm -f $(BINDIR)/audio-engine $(BINDIR)/radio-backend
	-systemctl daemon-reload 2>/dev/null || true
	@echo "    卸载完成（配置文件已保留在 $(CONFDIR)）"

# =============================================================================
# 服务管理
# =============================================================================

start:
	systemctl start redis audio-engine radio-backend

stop:
	systemctl stop radio-backend audio-engine

restart:
	systemctl restart audio-engine radio-backend

status:
	systemctl status audio-engine radio-backend

# =============================================================================
# 客户端静态文件
# =============================================================================

client:
	@echo "==> 正在复制客户端到静态资源目录..."
	mkdir -p $(DATADIR)/static
	cp ../radio-backend/static/index.html $(DATADIR)/static/
	@echo "    完成: $(DATADIR)/static/index.html"
