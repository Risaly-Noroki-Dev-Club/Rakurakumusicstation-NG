# Rakuraku Music Station NG — *The Show Must Go On*

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey.svg)
![C++](https://img.shields.io/badge/C++-17-orange.svg)
![Version](https://img.shields.io/badge/version-The%20Show%20Must%20Go%20On-ff69b4.svg)

**Languages / 语言**: [English](#english) · [中文](#中文)

> **"NG" = Nijika Generation** — a tribute to Nijika Ijichi from *Bocchi the Rock!*, reflecting the spirit of sharing music joyfully.
>
> **"The Show Must Go On"** — because no matter what, the music never stops.

---

## English

### Overview

Rakuraku Music Station NG is a **C++ streaming radio broadcast server** built from the ground up. One FFmpeg decoder feeds a lock-free ring buffer; a Linux `epoll` loop fans the audio bytes out to every connected listener simultaneously. A Crow-based web panel handles playlist management, file uploads, playback control, batch music downloads, and full system settings — all from the browser.

### How It Works

```
                           ┌──────────────────────┐
  media/   ──FFmpeg──▶     │   BroadcastBuffer    │  ──epoll──▶  N listeners
  (files)   (pipe)         │  (lock-free, 512 KB)  │  (fan-out)  (HTTP /stream)
                           └──────────────────────┘
                                    ▲
                          AudioPlayer (fork+exec ffmpeg)
                                    │
  Browser  ◀── Crow HTTP ──▶  WebServer (port 2240)
                                     │
                           SessionManager (cookie auth)
                           playlist_ / playlist_metadata_ (parallel vectors)

                           ┌──────────────────────┐
  Browser  ◀── Axum HTTP ─▶│   Rust Backend       │── Redis ──▶ Audio Engine
          (port 2241)       │  (REST + WebSocket)  │   pub/sub
                           └──────────────────────┘
```

A single FFmpeg child process decodes the current track with `-re` (real-time) and pipes raw MP3 bytes into the ring buffer. The `StreamServer`'s epoll thread reads from the buffer and writes to every connected TCP socket without blocking, using non-blocking I/O and edge-triggered epoll. When a track ends or a skip is requested, the `AudioPlayer` spawns a new FFmpeg process for the next track.

### Features

- **Radio-style broadcast** — single decoder, shared ring buffer, all listeners hear the same stream in sync
- **Low-latency `epoll` I/O** — edge-triggered epoll loop fans audio to hundreds of concurrent listeners on one thread
- **Multi-format support** — MP3, WAV, FLAC, OGG, M4A, AAC (anything FFmpeg can decode)
- **Hot-reload playlist** — uploads appear instantly; filesystem changes detected on startup; no restart needed
- **Rich metadata** — title, artist, album, genre, year, track number, duration, embedded cover art, and lyrics per track (extracted via `ffmpeg -i`)
- **Cookie-based session auth** — `HttpOnly` + `SameSite=Lax` cookies with `RAND_bytes` session IDs and `CRYPTO_memcmp` password verification; 24-hour sliding expiry
- **Optional guest skip** — configurable via `allow_guest_skip` in settings
- **Themeable UI** — station name, subtitle, primary/secondary/background colors from `settings.json`; changes take effect on page reload
- **PWA support** — `manifest.json` with theme color, `sw.js` service worker with network-first caching, mobile-optimized responsive layout
- **Batch music download** — NetEase Cloud Music (via `pyncm`), YouTube and Bilibili (via `yt-dlp`), from `.txt` or `.csv` playlists; with quality selection (standard / high / exhigh / lossless) and format conversion
- **Web settings panel** — edit all configuration live in the admin panel without touching `settings.json` manually; includes NetEase credential test
- **One-shot release build** — `build_release.sh` auto-detects distro, installs system deps, embeds all HTML/JS templates, compiles with LTO, and produces `dist/` ready to run
- **Persistent playlist order** — track ordering saved to `playlist_order.json`, survives restarts
- **Disk-override templates** — place custom HTML in `dist/templates/` to override embedded templates without rebuilding

### Requirements

- **Linux** — Arch, Debian, or Ubuntu (build script auto-detects)
- **GCC/G++ 7+** with C++17 support
- **System packages**: `build-essential` (or `base-devel`), `ffmpeg`, `libssl-dev` (or `openssl`), `libasio-dev` (or `asio`), `wget`, `curl`, `python3`
- **Python 3.8+** with `pip install -r requirements.txt` (for the batch download feature)
- **`crow_all.h`** — the Crow header-only C++ web framework (download once before building; see below)

### Quick Start

```bash
# 1. Download Crow (one-time)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. Build
./build_release.sh

# 3. Add audio files
cp /path/to/music/*.mp3 dist/media/

# 4. Change the admin password
#    Edit dist/settings.json → "admin_password"

# 5. Run
cd dist && ./start.sh
```

Open <http://localhost:2240> in a browser.  
Audio stream: <http://localhost:2240/stream>  
Admin panel: <http://localhost:2240/panel> (login at `/admin`)

Stop with `./stop.sh`.

### Manual Build (Development)

```bash
# Debian/Ubuntu deps
sudo apt-get install build-essential ffmpeg libssl-dev libasio-dev wget curl locales

# Arch deps
sudo pacman -S base-devel ffmpeg openssl asio wget curl

# Download Crow header (one-time)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# Debug build (warnings visible)
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.

# Release build (manual equivalent of build_release.sh)
g++ radioserver.cpp metadata.cpp -o dist/radioserver \
    -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto -I. -w
```

> **`crow_all.h` is a build prerequisite.** The release build script does **not** download it — fetch it once before building (see the `wget` command above). Delete the file to pull a newer version.

### Configuration — `dist/settings.json`

```json
{
    "station_name": "Rakuraku Music Station",
    "subtitle": "Your tagline here",
    "primary_color": "#764ba2",
    "secondary_color": "#667eea",
    "bg_color": "#f4f4f9",
    "admin_password": "change_me",
    "allow_guest_skip": false,
    "ncm_phone": "",
    "ncm_password": "",
    "ncm_cookie": ""
}
```

| Key | Default | Notes |
|-----|---------|-------|
| `station_name` | `"Rakuraku Music Station"` | Shown in page titles and headers |
| `subtitle` | `""` | Tagline displayed on listener page |
| `primary_color` | `"#764ba2"` | UI accent color (buttons, headers) |
| `secondary_color` | `"#667eea"` | UI gradient companion color |
| `bg_color` | `"#f4f4f9"` | Page background color |
| `admin_password` | `"admin123"` | **Change before exposing publicly** |
| `allow_guest_skip` | `false` | If `true`, unauthenticated users can call `/api/next` and `/api/prev` |
| `ncm_phone` | `""` | NetEase Cloud Music phone number |
| `ncm_password` | `""` | NetEase Cloud Music password |
| `ncm_cookie` | `""` | NetEase Cloud Music browser cookie (takes precedence over phone+password) |

All settings are editable from the web admin panel (`/panel` → Settings tab). Changes are written to disk immediately and take effect on the next page load.

### Playlist Persistence — `dist/playlist_order.json`

Tracks are ordered by a saved JSON array of filenames on startup. New files (not in the saved order) are appended alphabetically. The file is rewritten on every playlist mutation (upload, delete, batch download completion).

```json
["Adele - Hello.mp3", "Queen - Bohemian Rhapsody.flac", "..." ]
```

### HTTP API

**Public** (no auth required)

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Listener page (`index.html` with template substitution); shows admin controls if session cookie present |
| `GET` | `/api/playlist` | JSON: `{"playlist": [...filenames], "current": index, "metadata": [{TrackMetadata...}]}` |
| `GET` | `/api/stats` | JSON: `{"clients": count}` — current listener count |
| `GET` | `/api/metadata/<idx>` | Full metadata for track `idx` (title, artist, album, genre, year, track#, duration, has_cover, has_lyrics) |
| `GET` | `/api/cover/<idx>` | Binary JPEG of embedded cover art (404 if none) |
| `GET` | `/api/lyrics/<idx>` | Plain text lyrics (404 if none) |
| `GET` | `/stream` | **Audio stream**: takes over HTTP socket, sends `Content-Type: audio/mpeg`, registers with epoll fan-out |
| `GET` | `/manifest.json` | PWA web manifest (with template substitution) |
| `GET` | `/sw.js` | PWA service worker (network-first caching, bypasses `/stream` and `/api/*`) |
| `GET` | `/icon.svg` | Inline SVG favicon (purple music note) |

**Auth pages**

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/admin` | Login page; redirects to `/panel` if already authenticated |
| `GET` | `/panel` | Admin panel; redirects to `/admin` if no valid session |

**Auth actions**

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/admin/login` | Body: `{"password":"..."}` → sets `session_id` cookie (HttpOnly, SameSite=Lax, Max-Age=86400). Returns 200 or 401 |
| `POST` | `/admin/logout` | Destroys session, clears cookie |

**Admin** (session cookie required; `/api/next`, `/api/prev`, `/api/play/<idx>` also respect `allow_guest_skip`)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/upload` | Multipart file upload (binary-safe parser, ≤ 50 MB, supported formats); extracts metadata, adds to playlist |
| `POST` | `/api/next` | Skip to next track |
| `POST` | `/api/prev` | Skip to previous track |
| `POST` | `/api/play/<idx>` | Jump to track index `idx` |
| `POST` | `/api/delete/<idx>` | Delete track `idx` (removes file from disk, adjusts current index, saves `playlist_order.json`) |
| `POST` | `/admin/download` | Start batch download. Body: `{"playlist":"text","quality":"exhigh","format":"mp3"}`. Spawns `music_dl.py` in background. Returns `{"status":"started"}` |
| `GET` | `/admin/download/status` | Poll download progress: `{"running":bool, "log":"..."}` |
| `GET` | `/admin/settings/get` | Read all settings (passwords masked as `"********"`) |
| `POST` | `/admin/settings/save` | Save settings (updates both file and in-memory config) |
| `GET` | `/admin/settings/ncm` | Read NetEase config status |
| `POST` | `/admin/settings/ncm` | Save NetEase credentials (cookie or phone+password) |
| `POST` | `/admin/settings/ncm/test` | Test NetEase login by running `music_dl.py --verify-login` |

### Architecture

#### Core Classes (all in `radioserver.cpp`)

| Class | Role | Thread Model |
|-------|------|-------------|
| `RadioServer` | Top-level coordinator: owns all subsystems, handles init/start/stop lifecycle, signal handling | Main thread |
| `BroadcastBuffer` | Lock-free ring buffer (power-of-2 capacity = 512 KB). Single producer, multiple consumers. Atomic reads via `acquire`/`release` ordering. | Mutex-guarded writes; lock-free reads |
| `StreamServer` | Epoll loop fanning audio bytes to listeners. Two threads: `worker_loop` (accept new connections, detect disconnects) + `broadcast_loop` (read buffer, write to all sockets). | `clients_mutex_` guards client map |
| `ClientConnection` | Per-listener TCP socket (non-blocking). Sends `HTTP 200` + `Content-Type: audio/mpeg` header, then streams raw MP3 chunks from the buffer. | Per-connection, shutdown via atomic flag |
| `AudioPlayer` | Spawns FFmpeg child process: `ffmpeg -re -i <file> -vn -c:a libmp3lame -b:a 128k -ar 44100 -ac 2 -f mp3 pipe:1`. Uses `fork()`/`execvp()` (no shell) to avoid injection. Reads decoded bytes via `poll()`, pushes to buffer. | Single worker thread; `skip_track_` atomic flag; `idle_cv_` for playlist-empty wait |
| `WebServer` | Crow HTTP app serving UI + REST APIs. All ~25 routes registered in `setup_routes()`. Handles template rendering, session auth, file uploads, download subprocess management. | Crow's internal thread pool |
| `SessionManager` | In-memory session store (`unordered_map`). 32-char hex session IDs via `RAND_bytes`. Constant-time password comparison via `CRYPTO_memcmp`. 24-hour sliding expiry. | `std::mutex` with `lock_guard` |

#### Supporting Files

| File | Role |
|------|------|
| `metadata.hpp` / `metadata.cpp` | `TrackMetadata` struct + `MetadataManager` class. Extracts title/artist via filename regex (`Artist - Title` pattern), duration via `ffmpeg -i` (fork+exec into pipe), embedded cover art and lyrics via `ffprobe`. Supports MP3/WAV/FLAC/OGG/M4A/AAC. |
| `sessionmanager.hpp` | `SessionManager` class — self-contained header-only session store. |

#### Key Design Invariants

1. **Parallel vectors**: `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are always mutated together under `playlist_mutex_`. Any change to one must be mirrored on the other.
2. **BroadcastBuffer capacity** must be a power of two (enforced at construction).
3. **Template lookup order**: CWD → `templates/` subdirectory → embedded in `embedded_templates.hpp`.
4. **Port allocation**: C++ Crow server on 2240, Rust backend on 2241.
5. **Fork safety**: FFmpeg child processes use direct `fork()`/`execvp()` — no shell expansion, safe for filenames with special characters.
6. **`secrets.json` and `settings.json`** contain credentials — never commit them.

#### Startup Flow

1. `RadioServer` constructor: scans `./media/` for audio files, loads `playlist_order.json` for ordering, extracts metadata for each file via `MetadataManager`, picks random starting index.
2. `RadioServer::start()`: creates `StreamServer` (owns `BroadcastBuffer`), `AudioPlayer`, and `WebServer` in sequence.
3. `StreamServer::start()`: creates epoll fd, spins two threads (worker + broadcast).
4. `AudioPlayer::start()`: spins worker thread that loops: wait for non-empty playlist → fork FFmpeg → pipe decoded audio into buffer → poll for skip/EOF.
5. `WebServer::start()`: registers all routes, runs Crow multithreaded on port 2240.
6. Main thread enters `wait_for_shutdown()`. On `SIGINT`/`SIGTERM`, an atomic flag is set; cleanup stops WebServer → AudioPlayer → StreamServer.

### Project Layout

```
├── radioserver.cpp          # Main server — all core classes (2027 lines)
├── metadata.hpp             # TrackMetadata struct, MetadataManager declaration
├── metadata.cpp             # Metadata extraction implementation
├── sessionmanager.hpp       # In-memory session store
├── crow_all.h               # Crow header-only web framework (downloaded separately)
├── embedded_templates.hpp   # Build artifact — embedded HTML/JS/SW templates
│
├── build_release.sh         # One-shot release build script (324 lines)
├── music_dl.py              # Batch music downloader (NetEase / YouTube / Bilibili)
├── requirements.txt         # Python dependencies (pyncm, yt-dlp, requests)
│
├── index.html               # Listener page template (with {{VAR}} substitution)
├── panel.html               # Admin panel template
├── login.html               # Login page template
├── manifest.json            # PWA web manifest template
├── sw.js                    # PWA service worker
│
├── AGENTS.md                # Guidance for AI-assisted development sessions
├── LICENSE                  # MIT license
├── README.md                # This file
│
├── dist/                    # Runtime deployment directory (preserved across rebuilds)
│   ├── radioserver          # Compiled server binary
│   ├── start.sh             # Launch script (sets zh_CN.UTF-8 locale, backgrounds server)
│   ├── stop.sh              # Graceful shutdown (SIGTERM → SIGKILL after 10s)
│   ├── settings.json        # Runtime configuration
│   ├── playlist_order.json  # Persisted playlist ordering
│   ├── music_dl.py          # Copied from root during build
│   ├── requirements.txt     # Copied from root during build
│   ├── server.log           # Runtime log
│   ├── .server.pid          # PID file for stop.sh
│   ├── media/               # Audio files directory (gitignored)
│   └── templates/           # Optional custom template overrides (override embedded)
│
├── radio-backend/           # Rust REST + WebSocket API server (Axum, SQLite, Redis pub/sub)
│   ├── src/
│   │   └── routes/          # API handlers: auth, songs, queue, playlist, admin, favorites
│   ├── migrations/          # SQLite migration files
│   ├── config.toml.example  # Config template (port 2241, station, redis, jwt, queue)
│   └── static/
│       ├── index.html       # Player UI (HTML structure)
│       ├── style.css        # Player + admin panel styles
│       ├── app.js           # Player + admin panel logic
│       ├── manifest.json    # PWA Web Manifest
│       ├── sw.js            # PWA Service Worker
│       └── FRONTEND.md      # Frontend design spec
├── audio-engine/            # C++ audio decoding & streaming engine (FFmpeg, Redis client)
├── deploy/                  # Systemd deployment manifests (radio-backend, audio-engine, nginx)
├── paper-radio-plugin/      # [Deprecated] Plugin placeholder
└── Rakurakumusicstation-NG/ # Legacy nested repo (migration cruft — has its own .git)
```

### Batch Download — `music_dl.py`

The server launches `music_dl.py` as a subprocess when `/admin/download` is called. The script:

1. Reads a playlist file (one query per line, or CSV with artist/title columns)
2. Searches the configured source (NetEase Cloud Music, YouTube, or Bilibili)
3. Interactively (or non-interactively) selects tracks from search results
4. Downloads audio files to the media directory
5. On completion, the server reloads the playlist and saves `playlist_order.json`

**Quality options**: `standard` (128k), `high` (192k), `exhigh` (320k), `lossless` (FLAC).

**NetEase credentials**: set a browser cookie in settings, or phone+password. Use the test button in the admin panel to verify.

### Rust Backend — `radio-backend/`

A Rust business-logic backend built with **Axum** + **SQLite** (SQLx) + **Redis pub/sub**. Handles REST API, WebSocket broadcasting, user authentication (JWT + argon2), queue management, and LRC lyrics parsing. Communicates with the C++ audio engine through Redis pub/sub channels (`playback_state`, `command`, `queue_event`).

| Module | Role |
|--------|------|
| `routes/` | Axum handlers: `auth.rs` (login/logout), `songs.rs` (media CRUD), `queue.rs` (playback queue), `playlist.rs`, `admin.rs` |
| `websocket.rs` | WebSocket endpoint + Redis subscriber that forwards `playback_state` to connected browsers in real time |
| `db.rs` | SQLx connection pool, `AppState` initialization, migration runner |
| `auth.rs` | JWT token creation/validation, argon2 password hashing |
| `lyrics.rs` | LRC lyrics timestamp parsing |
| `queue_manager.rs` | In-memory playback queue with rate limiting |
| `models.rs` | Serde structs: Song, PlaybackState, QueueItem, etc. |

**Configuration**: `config.toml` (or env vars with `RADIO_*` prefix). Supports SQLite (default) or PostgreSQL. Default HTTP port: **2241**.

**Database migrations**: `migrations/001_initial_schema.sql` creates tables for songs, users, tokens, and queue history. `002_seed_defaults.sql` seeds a default admin user.

```bash
cd radio-backend
cp config.toml.example config.toml   # edit as needed
cp .env.example .env                 # or use env vars
cargo build --release
./target/release/radio-backend
```

### Audio Engine — `audio-engine/`

A C++17 audio streaming engine that wraps the main server's core (`radioserver.cpp` + `metadata.cpp`) with a **Redis pub/sub client** for cross-service communication with the Rust backend. Decodes files via FFmpeg, pushes PCM to a ring buffer, fans audio to listeners via epoll, and reports playback state (playing/paused/skipped, current track, position) to Redis.

| File | Role |
|------|------|
| `redis_client.hpp` | Lightweight hiredis wrapper — publish/subscribe with callback-based API |
| `playback_reporter.hpp` | Emits JSON playback state to the `playback_state` Redis channel on every track change |
| `file_server.hpp` | Serves raw media files over HTTP (for the Rust backend's "play from URL" feature) |
| `config.hpp` | TOML-subset parser with `AUDIO_*` env var overrides for Redis host/port, stream port, bitrate |
| `INTEGRATION_GUIDE.cpp` | Reference code showing integration between C++ audio server, Redis, and Rust backend |

```bash
cd audio-engine
# Install hiredis: sudo apt-get install libhiredis-dev
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
./audio_engine
```

Both `radio-backend/` and `audio-engine/` share the same Redis instance and communicate through pub/sub. The Rust backend provides the user-facing API + Web UI, while the audio engine handles the actual streaming.

### Frontend Architecture — `radio-backend/static/`

Split into three files served by the Rust backend (port 2241):

- **`index.html`** (280 lines) — HTML structure with tab navigation (Player, Queue, Library, Admin)
- **`style.css`** (206 lines) — All CSS via custom properties; supports light/dark themes and admin panel styles
- **`app.js`** (1130 lines) — All client logic: WebSocket, audio playback, lyrics, queue, search, auth, admin panel

**Admin panel** (JWT-protected, admin role required) provides sub-tabs for:
- User management (ban/unban, audit logs)
- Song management (list, delete, rescan, skip controls)
- Music upload (multipart form, ≤ 100 MB)
- Batch download (via `music_dl.py`, with quality/format selection and live log)
- NetEase Cloud Music credentials (cookie or phone+password + login test)
- System settings (station name, theme colors, admin password)

**Color system**: All colors use CSS custom properties (`var(--primary)`, `var(--text)`, `var(--border)`, etc.) defined in `:root`. The backend's `/api/station` endpoint injects `--primary`, `--secondary`, `--bg` at runtime. **Never hardcode a hex color in CSS properties or inline styles.**

**Theme**: Three-mode toggle in the top-right corner — auto (follows OS `prefers-color-scheme`), light, dark. Persisted to `localStorage`.

**Communication**:
- WebSocket `ws://<host>:2241/ws` for real-time playback state (title, artist, position, `lyrics_line`, `lyrics_text`)
- HTTP REST on the same port (2241) — see API table below
- `lyrics_line` + `lyrics_text` are computed server-side by the Rust backend; the frontend only renders pre-parsed LRC

**Legacy templates** (`index.html`, `panel.html`, `login.html`): served by the C++ Crow server on port 2240. These use `{{VAR}}` template substitution. Admin panel features (upload, download, settings, NCM) have been migrated to the Rust frontend; the legacy templates are kept for backward compatibility during the transition period.

Full design spec: [`radio-backend/static/FRONTEND.md`](radio-backend/static/FRONTEND.md)

### Runtime

The server binary must run from inside `dist/` (or any directory containing `media/` and `settings.json`) — all paths are relative to the working directory.

```bash
cd dist
./start.sh    # backgrounds server, writes logs to server.log
./stop.sh     # graceful shutdown
tail -f server.log    # watch logs
```

`start.sh` sets `LANG=zh_CN.UTF-8` and `LC_ALL=zh_CN.UTF-8` to handle CJK filenames. If you see garbled file names or `revents=16` pipe errors, ensure these locales are generated (`sudo locale-gen zh_CN.UTF-8`).

### Troubleshooting

| Symptom | Fix |
|---------|-----|
| Garbled non-ASCII filenames | Run `sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`, or always use `./start.sh` |
| Port 2240 already in use | `ss -ltnp \| grep 2240`; edit `Config::WEB_PORT` in `radioserver.cpp` and rebuild |
| Port 2241 already in use | `ss -ltnp \| grep 2241`; edit `port` in `radio-backend/config.toml` or set `RADIO_SERVER_PORT` env var and restart |
| `revents=16` pipe error | Locale issue — see above |
| No audio | Confirm `ffmpeg` is on `PATH`; check file extension is supported (mp3/wav/flac/ogg/m4a/aac) |
| `crow_all.h` not found | Download it: see the wget command in Quick Start |
| Download fails | Run `pip install -r requirements.txt`; verify NetEase credentials via the admin panel test button |
| Build fails on Arch | Install `asio` instead of `libasio-dev`: `sudo pacman -S asio` |
| "No such file or directory" in media | Filenames with special characters — ensure zh_CN.UTF-8 locale is active |

Logs are written to `dist/server.log`.

### Security

- **Change `admin_password`** before exposing the server publicly (default is `admin123`).
- Sessions use `HttpOnly` + `SameSite=Lax` cookies with `RAND_bytes`-generated 32-char hex IDs. 24-hour sliding expiry with constant-time password comparison (`CRYPTO_memcmp`).
- **Put a TLS-terminating reverse proxy** (nginx, Caddy) in front for public deployments.
- Uploads are capped at **50 MB** and rejected for unsupported file extensions.
- FFmpeg subprocesses use `fork()`/`execvp()` (no shell) to prevent command injection via filenames.
- **Never commit `dist/settings.json` or `secrets.json`** — they contain credentials.

### Development

- **AI-assisted development**: See [`AGENTS.md`](AGENTS.md) for build commands, codebase gotchas, invariants, and conventions used in this repo.
- **No tests, no CI** — verification is manual: build, run in `dist/`, hit endpoints with curl or browser.
- **Only two C++ compilation units**: `radioserver.cpp` + `metadata.cpp`. Link with `-lpthread -lssl -lcrypto`. C++17 required.
- **`CLAUDE.md`** — gitignored; contains legacy notes. `AGENTS.md` is the canonical guidance file.

### License

MIT — see [LICENSE](LICENSE).

### Credits

- **知夏 (Zhixia)** — project collaborator
- **[pyncm](https://github.com/HexiNail/pyncm)** — NetEase Cloud Music Python API
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** — YouTube / Bilibili audio downloader
- **[Crow](https://github.com/CrowCpp/Crow)** — header-only C++ web framework
- **[FFmpeg](https://ffmpeg.org/)** — audio decoding, metadata extraction
- **[OpenSSL](https://www.openssl.org/)** — cryptographic primitives (session IDs, password comparison)
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — networking primitives
- Inspiration: *Bocchi the Rock!* — Nijika Ijichi (伊地知虹夏), the namesake of "NG" (Nijika Generation)

---

## 中文

### 概述

Rakuraku Music Station NG 是一个从零构建的 **C++ 流媒体广播服务器**。单一的 FFmpeg 解码器向无锁环形缓冲区写入数据；Linux `epoll` 循环将音频字节同步分发给所有在线听众。基于 Crow 的 Web 面板提供播放列表管理、文件上传、播放控制、批量音乐下载和完整的系统设置功能——全部在浏览器中完成。

### 工作原理

```
                           ┌──────────────────────┐
  media/   ──FFmpeg──▶     │   BroadcastBuffer    │  ──epoll──▶  N 个听众
  (音频文件)  (管道)        │  (无锁环形缓冲, 512KB)  │  (扇出)     (HTTP /stream)
                           └──────────────────────┘
                                    ▲
                          AudioPlayer (fork+exec ffmpeg)
                                    │
  浏览器 ◀── Crow HTTP ──▶  WebServer (端口 2240)
                                     │
                           SessionManager (Cookie 认证)
                           playlist_ / playlist_metadata_ (平行向量)

                           ┌──────────────────────┐
  浏览器 ◀── Axum HTTP ─▶ │   Rust 后端           │── Redis ──▶ 音频引擎
          (端口 2241)      │  (REST + WebSocket)   │  发布/订阅
                           └──────────────────────┘
```

单个 FFmpeg 子进程以 `-re`（实时模式）解码当前曲目，将原始 MP3 字节通过管道写入环形缓冲区。`StreamServer` 的 epoll 线程从缓冲区读取数据，使用非阻塞 I/O 和边缘触发 epoll 写入每个连接的 TCP 套接字。曲目结束或被跳过时，`AudioPlayer` 会为下一曲目创建新的 FFmpeg 进程。

### 特性

- **电台式广播** — 单一解码器、共享环形缓冲区，所有听众同步收听同一路流
- **低延迟 `epoll` I/O** — 边缘触发 epoll 循环在单线程上向数百并发听众推送音频
- **多格式支持** — MP3、WAV、FLAC、OGG、M4A、AAC（所有 FFmpeg 可解码的格式）
- **热重载播放列表** — 上传后立即生效；启动时检测文件系统变化；无需重启
- **丰富元数据** — 每首曲目的标题、艺术家、专辑、流派、年份、音轨号、时长、内嵌封面和歌词（通过 `ffmpeg -i` 提取）
- **Cookie 会话认证** — `HttpOnly` + `SameSite=Lax` Cookie，使用 `RAND_bytes` 生成会话 ID，`CRYPTO_memcmp` 进行恒定时间密码比对；24 小时滑动过期
- **可选游客切歌** — 通过 `allow_guest_skip` 配置
- **可主题化 UI** — 台名、副标题、主色/辅色/背景色均从 `settings.json` 读取；修改后刷新页面即生效
- **PWA 支持** — `manifest.json` 配合主题色，`sw.js` Service Worker 网络优先缓存策略，移动端响应式适配
- **批量音乐下载** — 网易云音乐（pyncm）、YouTube 和 Bilibili（yt-dlp），支持 `.txt` 或 `.csv` 播放列表；可选音质（standard / high / exhigh / lossless）和格式转换
- **网页端设置面板** — 在管理面板中实时编辑所有配置，无需手动修改文件；包含网易云凭据测试功能
- **一键发布构建** — `build_release.sh` 自动识别发行版、安装系统依赖、内嵌所有 HTML/JS 模板、启用 LTO 编译，生成可直接运行的 `dist/`
- **持久化播放顺序** — 曲目顺序保存至 `playlist_order.json`，重启后恢复
- **模板磁盘覆盖** — 将自定义 HTML 放入 `dist/templates/` 即可覆盖内嵌模板，无需重新编译

### 系统要求

- **Linux** — Arch / Debian / Ubuntu（构建脚本自动识别）
- 支持 **C++17** 的 **GCC/G++ 7+**
- **系统包**：`build-essential`（或 `base-devel`）、`ffmpeg`、`libssl-dev`（或 `openssl`）、`libasio-dev`（或 `asio`）、`wget`、`curl`、`python3`
- **Python 3.8+** 并执行 `pip install -r requirements.txt`（批量下载功能需要）
- **`crow_all.h`** — Crow 纯头文件 C++ Web 框架（请在构建前下载一次，见下文）

### 快速开始

```bash
# 1. 下载 Crow（一次性操作）
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. 构建
./build_release.sh

# 3. 放入音频文件
cp /path/to/music/*.mp3 dist/media/

# 4. 修改管理员密码
#    编辑 dist/settings.json → "admin_password"

# 5. 启动
cd dist && ./start.sh
```

浏览器访问 <http://localhost:2240>。  
音频流地址：<http://localhost:2240/stream>  
管理面板：<http://localhost:2240/panel>（在 `/admin` 登录）

使用 `./stop.sh` 停止服务。

### 手动构建（开发）

```bash
# Debian/Ubuntu 依赖
sudo apt-get install build-essential ffmpeg libssl-dev libasio-dev wget curl locales

# Arch 依赖
sudo pacman -S base-devel ffmpeg openssl asio wget curl

# 下载 Crow 头文件（一次性操作）
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 调试构建（可见编译警告）
g++ radioserver.cpp metadata.cpp -o radioserver \
    -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.

# 发布构建（手动等价于 build_release.sh）
g++ radioserver.cpp metadata.cpp -o dist/radioserver \
    -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto -I. -w
```

> **`crow_all.h` 是构建前提。**发布构建脚本**不会**自动下载。请在首次构建前手动下载（见上方 `wget` 命令）。需要更新版本时删除该文件即可。

### 配置 — `dist/settings.json`

```json
{
    "station_name": "Rakuraku Music Station",
    "subtitle": "你的副标题",
    "primary_color": "#764ba2",
    "secondary_color": "#667eea",
    "bg_color": "#f4f4f9",
    "admin_password": "change_me",
    "allow_guest_skip": false,
    "ncm_phone": "",
    "ncm_password": "",
    "ncm_cookie": ""
}
```

| 键 | 默认值 | 说明 |
|----|--------|------|
| `station_name` | `"Rakuraku Music Station"` | 显示在页面标题和头部 |
| `subtitle` | `""` | 听众页显示的副标题 |
| `primary_color` | `"#764ba2"` | UI 强调色（按钮、标题） |
| `secondary_color` | `"#667eea"` | UI 渐变辅助色 |
| `bg_color` | `"#f4f4f9"` | 页面背景色 |
| `admin_password` | `"admin123"` | **对外部署前务必修改** |
| `allow_guest_skip` | `false` | 为 `true` 时游客也可调用 `/api/next` 和 `/api/prev` |
| `ncm_phone` | `""` | 网易云音乐手机号 |
| `ncm_password` | `""` | 网易云音乐密码 |
| `ncm_cookie` | `""` | 网易云音乐浏览器 Cookie（优先于手机号+密码） |

所有设置均可通过网页管理面板（`/panel` → 设置标签页）修改。修改后立即写入磁盘，下次加载页面时生效。

### 播放列表持久化 — `dist/playlist_order.json`

启动时按保存的 JSON 文件名数组排列曲目。新文件（不在已保存顺序中）按字母顺序追加。每次播放列表变更（上传、删除、批量下载完成）都会重写该文件。

```json
["Adele - Hello.mp3", "Queen - Bohemian Rhapsody.flac", "..." ]
```

### HTTP API

**公开接口**（无需认证）

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/` | 听众页（携带模板变量替换的 `index.html`）；若有会话 Cookie 则显示管理控件 |
| `GET` | `/api/playlist` | JSON：`{"playlist": [...文件名], "current": 索引, "metadata": [{曲目元数据...}]}` |
| `GET` | `/api/stats` | JSON：`{"clients": 数量}` — 在线听众数 |
| `GET` | `/api/metadata/<idx>` | 曲目 `idx` 完整元数据（标题、艺术家、专辑、流派、年份、音轨号、时长、是否有封面、是否有歌词） |
| `GET` | `/api/cover/<idx>` | 内嵌封面 JPEG 图片（无封面返回 404） |
| `GET` | `/api/lyrics/<idx>` | 纯文本歌词（无歌词返回 404） |
| `GET` | `/stream` | **音频流**：接管 HTTP 套接字，发送 `Content-Type: audio/mpeg`，通过 epoll 扇出注册 |
| `GET` | `/manifest.json` | PWA Web Manifest（含模板变量替换） |
| `GET` | `/sw.js` | PWA Service Worker（网络优先缓存，绕过 `/stream` 和 `/api/*`） |
| `GET` | `/icon.svg` | 内联 SVG 图标（紫色音符） |

**认证页面**

| 方法 | 路径 | 说明 |
|------|------|------|
| `GET` | `/admin` | 登录页；若已认证则重定向到 `/panel` |
| `GET` | `/panel` | 管理面板；无有效会话则重定向到 `/admin` |

**认证操作**

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/admin/login` | 请求体：`{"password":"..."}` → 设置 `session_id` Cookie（HttpOnly, SameSite=Lax, Max-Age=86400）。返回 200 或 401 |
| `POST` | `/admin/logout` | 销毁会话，清除 Cookie |

**管理员接口**（需要会话 Cookie；`/api/next`、`/api/prev`、`/api/play/<idx>` 也支持 `allow_guest_skip` 配置）

| 方法 | 路径 | 说明 |
|------|------|------|
| `POST` | `/upload` | 文件上传（二进制安全解析器，≤ 50 MB，支持的格式）；提取元数据，添加到播放列表 |
| `POST` | `/api/next` | 切换到下一首 |
| `POST` | `/api/prev` | 切换到上一首 |
| `POST` | `/api/play/<idx>` | 跳转到索引 `idx` 的曲目 |
| `POST` | `/api/delete/<idx>` | 删除索引 `idx` 的曲目（从磁盘删除文件，调整当前索引，保存 `playlist_order.json`） |
| `POST` | `/admin/download` | 启动批量下载。请求体：`{"playlist":"文本","quality":"exhigh","format":"mp3"}`。后台运行 `music_dl.py`。返回 `{"status":"started"}` |
| `GET` | `/admin/download/status` | 轮询下载进度：`{"running":布尔值, "log":"..."}` |
| `GET` | `/admin/settings/get` | 读取所有设置（密码显示为 `"********"`） |
| `POST` | `/admin/settings/save` | 保存设置（同时更新文件和内存配置） |
| `GET` | `/admin/settings/ncm` | 读取网易云配置状态 |
| `POST` | `/admin/settings/ncm` | 保存网易云凭据（Cookie 或手机号+密码） |
| `POST` | `/admin/settings/ncm/test` | 通过运行 `music_dl.py --verify-login` 测试网易云登录 |

### 架构

#### 核心类（全部在 `radioserver.cpp` 中）

| 类 | 职责 | 线程模型 |
|----|------|----------|
| `RadioServer` | 顶层协调器：拥有所有子系统，处理初始化/启动/停止生命周期和信号处理 | 主线程 |
| `BroadcastBuffer` | 无锁环形缓冲区（2 的幂次容量 = 512 KB）。单生产者、多消费者。使用 `acquire`/`release` 顺序的原子操作读取。 | 互斥锁保护写入；无锁读取 |
| `StreamServer` | Epoll 循环向听众扇出音频字节。两个线程：`worker_loop`（接受新连接、检测断开）+ `broadcast_loop`（读取缓冲区、写入所有套接字）。 | `clients_mutex_` 保护客户端映射表 |
| `ClientConnection` | 每个听众的 TCP 套接字（非阻塞）。发送 `HTTP 200` + `Content-Type: audio/mpeg` 头，然后流式传输原始 MP3 数据块。 | 每连接独立；通过原子标志位关闭 |
| `AudioPlayer` | 启动 FFmpeg 子进程：`ffmpeg -re -i <文件> -vn -c:a libmp3lame -b:a 128k -ar 44100 -ac 2 -f mp3 pipe:1`。使用 `fork()`/`execvp()`（无 shell）避免注入。通过 `poll()` 读取解码字节，推入缓冲区。 | 单个工作线程；`skip_track_` 原子标志位；`idle_cv_` 等待播放列表非空 |
| `WebServer` | Crow HTTP 应用，提供 UI + REST API。`setup_routes()` 中注册全部约 25 个路由。处理模板渲染、会话认证、文件上传、下载子进程管理。 | Crow 内部线程池 |
| `SessionManager` | 内存会话存储（`unordered_map`）。通过 `RAND_bytes` 生成 32 字符十六进制会话 ID。通过 `CRYPTO_memcmp` 进行恒定时间密码比对。24 小时滑动过期。 | `std::mutex` + `lock_guard` |

#### 辅助文件

| 文件 | 职责 |
|------|------|
| `metadata.hpp` / `metadata.cpp` | `TrackMetadata` 结构体 + `MetadataManager` 类。通过文件名正则（`艺术家 - 标题` 模式）提取标题/艺术家，通过 `ffmpeg -i` 获取时长（fork+exec 到管道），通过 `ffprobe` 获取内嵌封面和歌词。支持 MP3/WAV/FLAC/OGG/M4A/AAC。 |
| `sessionmanager.hpp` | `SessionManager` 类 — 自包含的纯头文件会话存储。 |

#### 关键设计约束

1. **平行向量**：`playlist_`（文件名）和 `playlist_metadata_`（曲目元数据）必须始终在 `playlist_mutex_` 保护下同步变更。任一方的修改必须镜像反映到另一方。
2. **BroadcastBuffer 容量**必须为 2 的幂（构造时强制检查）。
3. **模板查找顺序**：当前目录 → `templates/` 子目录 → `embedded_templates.hpp` 内嵌。
4. **端口分配**：C++ Crow 服务器 2240，Rust 后端 2241。
5. **Fork 安全**：FFmpeg 子进程使用直接 `fork()`/`execvp()`——无 shell 展开，对包含特殊字符的文件名安全。
6. **`secrets.json` 和 `settings.json`** 包含凭据——绝对不要提交到仓库。

#### 启动流程

1. `RadioServer` 构造函数：扫描 `./media/` 音频文件，加载 `playlist_order.json` 确定顺序，通过 `MetadataManager` 为每个文件提取元数据，随机选择起始索���。
2. `RadioServer::start()`：依次创建 `StreamServer`（拥有 `BroadcastBuffer`）、`AudioPlayer` 和 `WebServer`。
3. `StreamServer::start()`：创建 epoll 文件描述符，启动两个线程（worker + broadcast）。
4. `AudioPlayer::start()`：启动工作线程，循环：等待播放列表非空 → fork FFmpeg → 将解码音频通过管道传入缓冲区 → 轮询跳过/EOF。
5. `WebServer::start()`：注册所有路由，以多线程模式在 2240 端口运行 Crow。
6. 主线程进入 `wait_for_shutdown()`。收到 `SIGINT`/`SIGTERM` 时设置原子标志位；清理顺序为 WebServer → AudioPlayer → StreamServer。

### 项目结构

```
├── radioserver.cpp          # 主服务器 — 所有核心类（2027 行）
├── metadata.hpp             # TrackMetadata 结构体、MetadataManager 声明
├── metadata.cpp             # 元数据提取实现
├── sessionmanager.hpp       # 内存会话存储
├── crow_all.h               # Crow 纯头文件 Web 框架（单独下载）
├── embedded_templates.hpp   # 构建产物 — 内嵌 HTML/JS/SW 模板
│
├── build_release.sh         # 一键发布构建脚本（324 行）
├── music_dl.py              # 批量音乐下载器（网易云 / YouTube / Bilibili）
├── requirements.txt         # Python 依赖（pyncm, yt-dlp, requests）
│
├── index.html               # 听众页模板（含 {{VAR}} 变量替换）
├── panel.html               # 管理面板模板
├── login.html               # 登录页模板
├── manifest.json            # PWA Web Manifest 模板
├── sw.js                    # PWA Service Worker
│
├── AGENTS.md                # AI 辅助开发指南
├── LICENSE                  # MIT 许可证
├── README.md                # 本文件
│
├── dist/                    # 运行时部署目录（重编译时保留）
│   ├── radioserver          # 编译后的服务器二进制文件
│   ├── start.sh             # 启动脚本（设置 zh_CN.UTF-8 区域、后台运行服务器）
│   ├── stop.sh              # 优雅关闭（SIGTERM → 10 秒后 SIGKILL）
│   ├── settings.json        # 运行时配置
│   ├── playlist_order.json  # 持久化播放顺序
│   ├── music_dl.py          # 构建时从根目录复制
│   ├── requirements.txt     # 构建时从根目录复制
│   ├── server.log           # 运行时日志
│   ├── .server.pid          # stop.sh 使用的 PID 文件
│   ├── media/               # 音频文件目录（gitignored）
│   └── templates/           # 可选的自定义模板覆盖
│
├── radio-backend/           # Rust REST + WebSocket API 服务器（Axum、SQLite、Redis 发布/订阅）
│   ├── src/
│   │   └── routes/          # API 路由：auth, songs, queue, playlist, admin, favorites
│   ├── migrations/          # SQLite 数据库迁移文件
│   ├── config.toml.example  # 配置模板（端口 2241, station, redis, jwt, queue）
│   └── static/
│       ├── index.html       # 播放器 UI（HTML 结构）
│       ├── style.css        # 播放器 + 管理面板样式
│       ├── app.js           # 播放器 + 管理面板逻辑
│       ├── manifest.json    # PWA Web Manifest
│       ├── sw.js            # PWA Service Worker
│       └── FRONTEND.md      # 前端设计规范
├── audio-engine/            # C++ 音频解码与流媒体引擎（FFmpeg、Redis 客户端）
├── deploy/                  # Systemd 部署清单（radio-backend、audio-engine、nginx）
├── paper-radio-plugin/      # [已弃用] 插件占位
└── Rakurakumusicstation-NG/ # 旧版嵌套仓库（迁移遗留 — 有自己的 .git）
```

### 批量下载 — `music_dl.py`

调用 `/admin/download` 时，服务器以后台子进程方式启动 `music_dl.py`。脚本流程：

1. 读取播放列表文件（每行一个搜索词，或含艺术家/标题列的 CSV）
2. 在配置的数据源中搜索（网易云音乐、YouTube 或 Bilibili）
3. 交互式（或非交互式）从搜索结果中选择曲目
4. 将音频文件下载到媒体目录
5. 完成后，服务器重新加载播放列表并保存 `playlist_order.json`

**音质选项**：`standard`（128k）、`high`（192k）、`exhigh`（320k）、`lossless`（FLAC）。

**网易云凭据**：在设置中填入浏览器 Cookie，或手机号+密码。使用管理面板中的测试按钮验证。

### Rust 后端 — `radio-backend/`

基于 **Axum** + **SQLite**（SQLx）+ **Redis 发布/订阅**构建的 Rust 业务逻辑后端。处理 REST API、WebSocket 广播、用户认证（JWT + argon2）、队列管理和 LRC 歌词解析。通过 Redis 发布/订阅频道（`playback_state`、`command`、`queue_event`）与 C++ 音频引擎进行跨服务通信。

| 模块 | 职责 |
|------|------|
| `routes/` | Axum 路由处理器：`auth.rs`（登录/登出）、`songs.rs`（媒体 CRUD）、`queue.rs`（播放队列）、`playlist.rs`、`admin.rs` |
| `websocket.rs` | WebSocket 端点 + Redis 订阅者，将 `playback_state` 实时转发给连接的浏览器 |
| `db.rs` | SQLx 连接池、`AppState` 初始化、数据库迁移运行器 |
| `auth.rs` | JWT 令牌创建/验证、argon2 密码哈希 |
| `lyrics.rs` | LRC 歌词时间戳解析 |
| `queue_manager.rs` | 带限速的内存播放队列 |
| `models.rs` | Serde 结构体：Song、PlaybackState、QueueItem 等 |

**配置**：`config.toml`（或通过 `RADIO_*` 前缀的环境变量）。支持 SQLite（默认）或 PostgreSQL。默认 HTTP 端口：**2241**。

**数据库迁移**：`migrations/001_initial_schema.sql` 创建歌曲、用户、令牌和队列历史记录等表。`002_seed_defaults.sql` 创建默认管理员用户。

```bash
cd radio-backend
cp config.toml.example config.toml   # 按需修改
cp .env.example .env                 # 或使用环境变量
cargo build --release
./target/release/radio-backend
```

### 音频引擎 — `audio-engine/`

一个 C++17 音频流媒体引擎，将主服务器的核心（`radioserver.cpp` + `metadata.cpp`）与 **Redis 发布/订阅客户端**封装在一起，用于与 Rust 后端进行跨服务通信。通过 FFmpeg 解码音频文件，将 PCM 推入环形缓冲区，通过 epoll 向听众扇出音频，并向 Redis 报告播放状态（播放中/暂停/跳过、当前曲目、播放位置）。

| 文件 | 职责 |
|------|------|
| `redis_client.hpp` | 轻量级 hiredis 封装——基于回调的发布/订阅 API |
| `playback_reporter.hpp` | 每次曲目切换时将 JSON 格式的播放状态发送到 Redis 的 `playback_state` 频道 |
| `file_server.hpp` | 通过 HTTP 提供原始媒体文件（供 Rust 后端的"通过 URL 播放"功能使用） |
| `config.hpp` | TOML 子集解析器，支持 `AUDIO_*` 环境变量覆盖 Redis 主机/端口、流媒体端口、比特率等 |
| `INTEGRATION_GUIDE.cpp` | 演示 C++ 音频服务器、Redis 客户端与 Rust 后端集成方式的参考代码 |

```bash
cd audio-engine
# 安装 hiredis：sudo apt-get install libhiredis-dev
mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
make -j$(nproc)
./audio_engine
```

`radio-backend/` 和 `audio-engine/` 共享同一个 Redis 实例，通过发布/订阅进行通信。Rust 后端提供面向用户的 API 和 Web UI，而音频引擎负责实际的音频流处理。

### 前端架构 — `radio-backend/static/`

已拆分为三个文件，由 Rust 后端在 2241 端口托管：

- **`index.html`**（280 行）— HTML 结构，包含标签页导航（播放、队列、曲库、管理）
- **`style.css`**（206 行）— 所有 CSS 样式，通过自定义属性管理；支持亮/暗主题及管理面板样式
- **`app.js`**（1130 行）— 所有客户端逻辑：WebSocket、音频播放、歌词、队列、搜索、认证、管理面板

**管理面板**（需 JWT 认证及管理员角色）提供以下子标签页：
- 用户管理（封禁/解封、操作日志）
- 歌曲管理（列表查看、删除、重新扫描、切歌控制）
- 音乐上传（multipart 表单，≤ 100 MB）
- 批量下载（通过 `music_dl.py`，支持音质/格式选择和实时日志）
- 网易云音乐凭据（Cookie 或手机号+密码 + 登录测试）
- 系统设置（站点名称、主题颜色、管理员密码）

**色彩系统**：所有颜色通过 CSS 自定义属性（`var(--primary)`、`var(--text)`、`var(--border)` 等）引用，在 `:root` 中定义。后端 `/api/station` 接口在运行时注入 `--primary`、`--secondary`、`--bg`。**严禁在 CSS 属性或内联 style 中直接写色码。**

**主题**：右上角三态切换按钮 — 自动（跟随系统 `prefers-color-scheme`）、亮色、暗色。持久化到 `localStorage`。

**通信方式**：
- WebSocket `ws://<host>:2241/ws` 实时推送播放状态（曲名、艺术家、播放进度、`lyrics_line`、`lyrics_text`）
- HTTP REST 同端口（2241）—— 见上文 API 表
- `lyrics_line` 和 `lyrics_text` 由 Rust 后端服务端计算和注入，前端仅负责渲染预解析的 LRC 歌词

**遗留模板**（`index.html`、`panel.html`、`login.html`）：由 C++ Crow 服务器在 2240 端口托管，使用 `{{VAR}}` 模板变量替换。管理面板功能（上传、下载、设置、网易云）已迁移至 Rust 前端，遗留模板在过渡期间保留以保持向后兼容。

完整设计规范：[`radio-backend/static/FRONTEND.md`](radio-backend/static/FRONTEND.md)

### 运行时

服务器二进制文件必须从 `dist/`（或任何包含 `media/` 和 `settings.json` 的目录）中运行——所有路径相对于工作目录。

```bash
cd dist
./start.sh    # 后台启动服务器，日志写入 server.log
./stop.sh     # 优雅关闭
tail -f server.log    # 查看日志
```

`start.sh` 设置 `LANG=zh_CN.UTF-8` 和 `LC_ALL=zh_CN.UTF-8` 以处理中日韩文件名。如果看到乱码或 `revents=16` 管道错误，请确保已生成这些区域设置（`sudo locale-gen zh_CN.UTF-8`）。

### 故障排除

| 现象 | 解决方法 |
|------|----------|
| 非 ASCII 文件名乱码 | 运行 `sudo locale-gen zh_CN.UTF-8 && sudo update-locale LANG=zh_CN.UTF-8`，或始终使用 `./start.sh` |
| 2240 端口被占用 | `ss -ltnp \| grep 2240`；修改 `radioserver.cpp` 中的 `Config::WEB_PORT` 并重新构建 |
| 2241 端口被占用 | `ss -ltnp \| grep 2241`；修改 `radio-backend/config.toml` 中的 `port` 或设置 `RADIO_SERVER_PORT` 环境变量后重启 |
| `revents=16` 管道错误 | Locale 问题 — 参考上一条 |
| 没有声音 | 确认 `ffmpeg` 在 `PATH` 中；检查文件扩展名是否受支持（mp3/wav/flac/ogg/m4a/aac） |
| `crow_all.h` 未找到 | 下载：参见快速开始中的 wget 命令 |
| 下载失败 | 运行 `pip install -r requirements.txt`；通过管理面板测试按钮验证网易云凭据 |
| Arch 上构建失败 | 安装 `asio` 而非 `libasio-dev`：`sudo pacman -S asio` |
| media 中出现"No such file or directory" | 文件名含特殊字符 — 确保 zh_CN.UTF-8 区域已激活 |

日志文件位于 `dist/server.log`。

### 安全建议

- 对外部署前**务必修改 `admin_password`**（默认值为 `admin123`）。
- 会话使用 `HttpOnly` + `SameSite=Lax` Cookie，会话 ID 由 `RAND_bytes` 生成 32 字符十六进制字符串。24 小时滑动过期，使用恒定时间密码比对（`CRYPTO_memcmp`）。
- 公网部署请在前面挂 **TLS 终止反向代理**（nginx、Caddy）。
- 上传限制 **50 MB**，不支持的文件扩展名会被拒绝。
- FFmpeg 子进程使用 `fork()`/`execvp()`（无 shell），防止通过文件名进行命令注入。
- **切勿提交 `dist/settings.json` 或 `secrets.json`**——它们包含凭据。

### 开发

- **AI 辅助开发**：参见 [`AGENTS.md`](AGENTS.md) 了解构建命令、代码库注意事项、约束和本仓库使用的约定。
- **无测试、无 CI** — 验证方式为手动：构建，在 `dist/` 中运行，通过 curl 或浏览器测试接口。
- **仅两个 C++ 编译单元**：`radioserver.cpp` + `metadata.cpp`。链接参数：`-lpthread -lssl -lcrypto`。需 C++17。
- **`CLAUDE.md`** — 已被 gitignore；包含旧版笔记。`AGENTS.md` 是规范的指导文件。

### 许可证

MIT — 详见 [LICENSE](LICENSE)。

### 致谢

- **知夏 (Zhixia)** — 项目协作者
- **[pyncm](https://github.com/HexiNail/pyncm)** — 网易云音乐 Python API 封装
- **[yt-dlp](https://github.com/yt-dlp/yt-dlp)** — YouTube / Bilibili 音频下载器
- **[Crow](https://github.com/CrowCpp/Crow)** — 纯头文件 C++ Web 框架
- **[FFmpeg](https://ffmpeg.org/)** — 音频解码与元数据提取
- **[OpenSSL](https://www.openssl.org/)** — 密码学原语（会话 ID 生成、密码比对）
- **[Boost](https://www.boost.org/) / [Asio](https://think-async.com/Asio/)** — 网络原语
- 灵感来源：《孤独摇滚！》伊地知虹夏 — "NG"（Nijika Generation）命名由来

---

<div align="center"><em>觉得有用的话欢迎点 Star ⭐ / Star the repo if you find it useful.</em></div>

<div align="center"><em>🎶 The Show Must Go On 🎶</em></div>
