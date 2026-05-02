# CLAUDE.md — Rakuraku Music Station NG

## Architecture

Two services communicate via Redis pub/sub:

- **C++ Audio Engine** (port 2240): ffmpeg → ring buffer → TCP stream. 2 endpoints: `/stream`, `/health`.
- **Rust Backend** (port 2241): Axum REST API + WebSocket + SQLite + JWT auth.

Redis channels: `playback_state` (C++→Rust), `command` (Rust→C++), `queue_event` (Rust→C++).

## Source Map

```
radioserver.cpp          # C++ — all 6 core classes in one file (~640 lines)
metadata.cpp/.hpp        # TrackMetadata struct + MetadataManager (ffprobe extraction)
third_party/hiredis/     # Redis C client (static library, compiled in-tree)
crow_all.h               # Crow header (download before build, gitignored)
build_release.sh          # One-shot release build

radio-backend/
├── config.toml.example   # Rust config template
├── static/               # Web UI (index.html, style.css, app.js, manifest.json, sw.js)
├── src/
│   ├── main.rs           # Axum bootstrap
│   ├── db.rs             # SQLite pool + Redis conn + AppState
│   ├── auth.rs           # JWT + argon2 + ban system
│   ├── config.rs         # TOML config loading
│   ├── error.rs          # AppError enum
│   ├── models.rs         # Serde structs (Song, PlaybackState, AudioCommand, etc.)
│   ├── lyrics.rs         # LRC parser
│   ├── websocket.rs      # /ws handler + Redis subscriber + publish_command
│   ├── queue_manager.rs  # Queue CRUD + rate limiting
│   └── routes/
│       ├── mod.rs        # Router assembly
│       ├── auth.rs       # /api/auth/*
│       ├── songs.rs      # /api/songs/*
│       ├── queue.rs      # /api/queue/*
│       ├── playlist.rs   # /api/playlists/*
│       ├── favorites.rs  # /api/favorites/*
│       └── admin.rs      # /api/admin/*
└── migrations/           # SQLx migrations
```

## Build

```bash
# Full build (C++ + Rust)
./build_release.sh

# C++ debug build
(cd third_party/hiredis && gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && ar rcs libhiredis.a *.o && rm -f *.o)
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis third_party/hiredis/libhiredis.a

# Rust build
cd radio-backend && cargo build --release
```

## Key Classes (radioserver.cpp)

| Class | Role |
|-------|------|
| `BroadcastBuffer` | Lock-free ring buffer (512 KB, power-of-2 capacity) |
| `ClientConnection` | Per-listener TCP socket (non-blocking, TCP_NODELAY) |
| `StreamServer` | Epoll loop fanning audio to clients |
| `AudioPlayer` | Forks ffmpeg → reads MP3 via pipe → pushes to buffer; publishes state to Redis every 500ms; processes Redis commands inline |
| `RedisBridge` | hiredis wrapper: sync publish + blocking subscribe in background thread |
| `WebServer` | Minimal Crow app: `/stream` + `/health` only |
| `RadioServer` | Orchestrator: init_playlist, start, stop, signal handling |

## Invariants

- `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are parallel vectors; mutate together under `playlist_mutex_`.
- `BroadcastBuffer` capacity must be power of two (checked at construction).
- C++ engine must run from a directory containing `media/`.
- Redis is optional for C++ engine — runs standalone if unavailable.
- `-lssl -lcrypto` required (Crow uses OpenSSL).
