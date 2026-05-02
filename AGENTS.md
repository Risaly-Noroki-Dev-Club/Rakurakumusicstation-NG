# AGENTS.md

This file provides guidance for OpenCode sessions working in this repository.

## Architecture

- **C++ Audio Engine** (`radioserver.cpp` + `metadata.cpp`) — port 2240
  - Audio pipeline: ffmpeg → `BroadcastBuffer` → `StreamServer` (TCP clients)
  - Endpoints: `/stream` (audio), `/health` (status)
  - Redis pub/sub: publishes `PlaybackState` to `playback_state` channel, subscribes to `command` channel
  - Self-contained playlist scanner from `./media/`
- **Rust Backend** (`radio-backend/`) — port 2241
  - JWT auth, multi-user, SQLite, playlist/queue management, WebSocket
  - Drives C++ engine via Redis `command` channel; receives state via Redis `playback_state` channel
  - Static web UI in `radio-backend/static/` (Vue 3 CDN SPA — no build tools)

## Build

```bash
./build_release.sh               # Full build (C++ + Rust, auto-downloads crow_all.h)
./build_release.sh --no-crow     # Skip crow_all.h download (fail if missing)
./build_release.sh --skip-rust   # C++ only

# Debug build — C++ audio engine
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis \
  third_party/hiredis/libhiredis.a

# Build hiredis (one-time)
(cd third_party/hiredis && gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && ar rcs libhiredis.a *.o && rm -f *.o)
```

- Only two C++ source files: `radioserver.cpp` + `metadata.cpp`.
- Link flags `-lssl -lcrypto` are required (`crow_all.h` uses them).
- C++17 is required.
- Release build adds `-O3 -flto -march=native -w`; debug build omits `-w`.

## Runtime

```bash
cd dist
./start.sh         # starts audio engine (nohup), writes .server.pid, logs to server.log
./stop.sh          # reads .server.pid, pgrep fallback cleanup
./start-rust.sh    # starts Rust backend (if present), writes .rust-server.pid
./stop-rust.sh     # stops Rust backend
```

- The C++ engine must run from inside `dist/` (or any dir containing `media/`).
- `build_release.sh` preserves `dist/media/` and `dist/playlist_order.json` across rebuilds.
- `playlist_order.json` is created at runtime on first run; no seed file exists.
- `.server.pid` and `.rust-server.pid` are the daemon lifecycle mechanism.

## Key invariants

- `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are parallel vectors; any mutation must be mirrored under `playlist_mutex_`.
- `BroadcastBuffer` capacity must be a power of two (enforced at construction via `throw`).
- All core classes (`RadioServer`, `BroadcastBuffer`, `StreamServer`, `AudioPlayer`, `WebServer`, `RedisBridge`, `ClientConnection`) live in `radioserver.cpp`.

## Known quirks

### C++ engine
- `crow_all.h` and `radioserver` (compiled binary at repo root) are build artifacts; delete before committing.
- `dist/settings.json` contains credentials; never commit it.
- Redis is optional — the engine runs standalone if unavailable.

### Rust backend
- **Redis is optional** — the backend starts fine without it but loses C++ engine command forwarding.
- **No Axum auth middleware** — every protected handler calls `require_auth_from_headers()` manually.
- **JWT secret defaults to a hardcoded dev value** (`radio-backend-dev-secret-change-in-production`). Must override via `config.toml` `[jwt].secret` or `RADIO_JWT_SECRET` env var.
- **Admin seed password is a placeholder** — the hash in `migrations/002_seed_defaults.sql` is all `A` characters. The admin user cannot log in until a real password hash is inserted or the register endpoint is used.
- **Settings save but don't hot-reload** — `POST /api/admin/settings` writes `config.toml` but changes take effect only after restart.
- **Rescan needs `ffprobe`** on PATH to extract audio duration metadata.
- **Download feature needs `music_dl.py`** at a path configured via `MUSIC_DL_PATH` env var.
- **SQLite-only** — migrations use `AUTOINCREMENT`, `datetime('now')`, `INSERT OR IGNORE`. PostgreSQL is noted in comments but requires migration rewrite.
- **Migrations run automatically** at startup via `sqlx::migrate!`, no manual step needed.
- **Static files are a fallback** (`ServeDir::new("static")` as `.fallback_service()`) — any unmatched route falls through to the SPA, enabling client-side routing.
- **`now_playing` HTTP endpoint is DB-only** (no real-time position_ms). Real-time playback data comes via WebSocket only.
- **`stream_base` config** (`[audio_engine].stream_base`) accepts absolute URLs (`http://...`) or relative paths (`/stream`). Relative paths are resolved against `base_url`. The frontend reads the resolved URL from `/api/station`.
- **Missing cover art returns a placeholder SVG** — the `/api/songs/{id}/cover` endpoint returns a default music-note icon when no cover exists, rather than a 404 error.

## No tests / no CI

- No unit tests, no lint/typecheck config, no CI workflows.
- Manual verification: build, run in `dist/`, hit API endpoints with curl or browser.
- Server logs to `dist/server.log`.
