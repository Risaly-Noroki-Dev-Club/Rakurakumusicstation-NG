# AGENTS.md

This file provides guidance for OpenCode sessions working in this repository.

## Architecture (v3.0)

- **Radio Engine** (`radio-engine/` crate) — embedded Rust audio engine
  - Audio pipeline: ffmpeg → `RingBuffer` → stream (TCP clients)
  - Provides: `RingBuffer`, `Player`, playlists, metadata extraction
  - Embedded as a local crate inside `radio-backend` (no separate process)
- **Rust Backend** (`radio-backend/`) — single binary, port 2241
  - Device-based auth (httpOnly `device_token` cookie), admin via `admin_setup_token`
  - SQLite, playlist/queue management, WebSocket, lyrics
  - Embeds `radio-engine` — no external C++ process, no HTTP inter-process communication
  - Static web UI in `radio-backend/static/` (Vue 3 SFC + Vite + TypeScript)
- **Legacy C++ Engine** (`legacy/cpp-engine/`) — archived, not built or used

## Build

```bash
./build_release.sh               # Full build (Rust backend + engine)
```

- Source files: `radio-engine/src/*.rs` (7 modules) + `radio-backend/src/*.rs` (9 modules).
- The build script requires `cargo` and `ffmpeg` (runtime dependency, not build-time).
- Build output: `dist/radio-backend` (single binary), `dist/static/`, `dist/start.sh`, `dist/stop.sh`.
- `dist/config.toml` is seeded from `radio-backend/config.toml.example` on first build.
- `dist/media/` and `dist/playlist_order.json` are preserved across rebuilds.

## Runtime

```bash
cd dist
./start.sh         # starts radio-backend (nohup), writes .server.pid
./stop.sh          # stops the service, reads PID file, pgrep fallback
```

- The server must run from inside `dist/` (or any dir containing `media/`).
- `.server.pid` is the daemon lifecycle mechanism (single process, single PID file).
- `playlist_order.json` is created at runtime on first run; no seed file exists.

## Key invariants

- `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are parallel vectors in the radio-engine player; any mutation must be mirrored under the playlist lock.
- `RingBuffer` capacity must be a power of two (enforced at construction).
- All core engine types (`RingBuffer`, `Player`, `StreamServer`) live in `radio-engine/src/`.
- The backend uses a `device_cookie_middleware` that auto-creates device users on first visit — every HTTP response sets or refreshes the httpOnly cookie.

## Known quirks

### Audio engine (radio-engine)
- The engine emits audio stream at `/stream` from the same HTTP server (port 2241), not a separate port.
- Playlist scanning uses `ffmpeg` for metadata extraction (same as legacy C++ engine).

### Rust backend
- **Device-based auth** — no passwords, no JWT. Each browser/device gets a `device_token` cookie. Admin is promoted via `admin_setup_token` (configure in `config.toml`).
- **No Redis dependency** — engine is embedded in-process.
- **No Axum auth middleware** — every protected handler calls `require_auth_from_headers()` or uses `AuthUser` extractor.
- **Admin setup token** — configured in `config.toml` `[device].admin_setup_token`. Must be set before first run.
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
