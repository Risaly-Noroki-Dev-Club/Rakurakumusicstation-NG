# Rakuraku Music Station NG — *The Long Last Goodbye*

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![C++](https://img.shields.io/badge/C++-17-orange.svg)
![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)

> **"The Long Last Goodbye"** — a farewell to the monolithic single-process era.  
> C++ handles the audio; Rust owns the web. Redis is the bridge.

---

## Architecture

```
media/  ──FFmpeg──▶  BroadcastBuffer  ──epoll──▶  N listeners
                     (lock-free ring)   (fan-out)  (HTTP /stream)
                            ▲
                    AudioPlayer (fork ffmpeg → pipe → buffer)
                            │ ╲
                Redis ──────┘  └────── Redis
               pub/sub               pub/sub
                  │                     │
            playback_state          command
                  │                     │
                  ▼                     ▼
         Rust Backend (port 2241) ◀──────────┘
         Axum + SQLite + JWT + WebSocket
                  │
         static/ (Web UI)
```

Two services, one Redis:

| Service | Port | Role |
|---------|------|------|
| **C++ Audio Engine** (`radioserver.cpp`) | 2240 | ffmpeg pipeline → ring buffer → TCP stream; publishes `PlaybackState` to Redis; subscribes to `command` channel |
| **Rust Backend** (`radio-backend/`) | 2241 | REST API, WebSocket, multi-user JWT auth, SQLite, queue/playlist management; drives C++ engine via Redis |

- Redis is **optional** for the C++ engine — it runs standalone if unavailable.
- Both services share the `media/` directory for audio files.

## Quick Start

```bash
# 1. Get crow_all.h (one-time)
wget $(curl -sf https://api.github.com/repos/CrowCpp/Crow/releases/latest \
  | python3 -c "import sys,json; r=json.load(sys.stdin); print(next(a['browser_download_url'] for a in r['assets'] if a['name']=='crow_all.h'))") \
  -O crow_all.h

# 2. Build
./build_release.sh

# 3. Add audio files
cp /path/to/music/*.mp3 dist/media/

# 4. Start audio engine
cd dist && ./start.sh

# 5. Start Rust backend (optional — for Web UI and API)
cd radio-backend
cp config.toml.example config.toml
cargo run --release
# Or: ./target/release/radio-backend
```

- Audio stream: `http://localhost:2240/stream`
- Health check: `http://localhost:2240/health`
- Web UI (Rust): `http://localhost:2241`

Stop: `./stop.sh` (C++) / `Ctrl+C` (Rust)

## Build (Manual)

```bash
# Dependencies: g++, ffmpeg, libssl-dev, libasio-dev, python3

# Build hiredis (one-time)
(cd third_party/hiredis && \
  gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && \
  ar rcs libhiredis.a *.o && rm -f *.o)

# Debug build
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis \
  third_party/hiredis/libhiredis.a

# Release build
g++ radioserver.cpp metadata.cpp -o dist/radioserver \
  -std=c++17 -O3 -flto -march=native -lpthread -lssl -lcrypto \
  -I. -Ithird_party/hiredis -w third_party/hiredis/libhiredis.a

# Rust backend
cd radio-backend && cargo build --release
```

## C++ Audio Engine — API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/stream` | Audio stream (`Content-Type: audio/mpeg`); takes over TCP socket |
| `GET` | `/health` | `{"status":"ok","service":"rakuraku-audio-engine","clients":N}` |

No auth, no admin panel — all management goes through the Rust backend.

## Rust Backend — API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/station` | No | Station info + theme colors |
| `GET` | `/api/now-playing` | No | Current track + `lyrics_line` / `lyrics_text` |
| `GET` | `/api/songs?q=` | No | Song library search |
| `GET` | `/api/songs/{id}/cover` | No | Cover art (JPEG/PNG) |
| `GET` | `/api/queue` | No | Shared queue |
| `GET` | `/api/queue/history` | No | Play history |
| `POST` | `/api/queue` | JWT | Add song to queue |
| `DELETE` | `/api/queue/{id}` | Admin | Remove queue item |
| `POST` | `/api/queue/skip` | Admin | Skip track (→ Redis `command`) |
| `POST` | `/api/auth/register` | No | Register |
| `POST` | `/api/auth/login` | No | Login → JWT |
| `GET` | `/api/auth/me` | JWT | Current user info |
| `GET` | `/api/favorites` | JWT | User favorites |
| `POST` `DELETE` | `/api/favorites/{id}` | JWT | Add/remove favorite |
| `GET` `POST` `DELETE` | `/api/playlists` | JWT | Playlist CRUD |
| `GET` `POST` `DELETE` | `/api/admin/*` | Admin | User management, stats, song upload/delete, settings, download, NCM |
| `WS` | `/ws` | No | WebSocket: real-time `playback_state` + `queue_update` |

## Redis Channels

| Channel | Direction | Payload |
|---------|-----------|---------|
| `playback_state` | C++ → Rust | `PlaybackState` JSON (song_id, file_path, position_ms, duration_ms, status, total_bytes_sent, bitrate_kbps, track_start_timestamp_ms) |
| `command` | Rust → C++ | `AudioCommand` JSON (`type`: skip/prev/play/stop, `file_path`, `song_id`) |
| `queue_event` | Rust → C++ | `QueueEvent` JSON (queue changes — for future use) |

## Configuration

### C++ Engine

No configuration file needed. Scans `media/` on startup. Playlist order persists in `playlist_order.json`.

### Rust Backend — `config.toml`

```toml
[server]          # host, port (default 2241)
[database]        # SQLite URL
[redis]           # URL, channel names
[audio_engine]    # C++ engine base_url, media_path
[jwt]             # secret, expiry_hours
[queue]           # max_size, rate_limit
[station]         # name, subtitle, theme colors
[logging]         # level
```

## Supported Formats

MP3, WAV, FLAC, OGG, M4A, AAC

## License

MIT

## Credits

- **知夏 (Zhixia)** — collaborator
- [Crow](https://github.com/CrowCpp/Crow) — C++ HTTP framework
- [hiredis](https://github.com/redis/hiredis) — C Redis client
- [FFmpeg](https://ffmpeg.org/) — audio decoding
- [Axum](https://github.com/tokio-rs/axum) — Rust HTTP framework
- [SQLx](https://github.com/launchbadge/sqlx) — Rust SQL toolkit
- Inspired by *Bocchi the Rock!* — Nijika Ijichi
