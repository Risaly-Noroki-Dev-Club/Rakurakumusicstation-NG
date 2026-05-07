# AGENTS.md

This file provides guidance for OpenCode sessions working in this repository.

## Architecture (v3.1-beta)

- **Radio Engine** (`radio-engine/` crate) — embedded Rust audio engine
  - Audio pipeline: ffmpeg → `RingBuffer` → stream (TCP clients)
  - Provides: `RingBuffer`, `Player`, `play_queue`, metadata extraction, `util` helpers
  - Embedded as a local crate inside `radio-backend` (no separate process)
  - **Key design**: all paths stored in `play_queue` are **relative to `media_path`**; use `util::resolve_media_path` / `relativize_media_path` for path normalization
- **Rust Backend** (`radio-backend/`) — single binary, port 2241
  - Device-based auth (httpOnly `device_token` cookie), admin via `admin_setup_token`
  - SQLite, play_queue/queue management, WebSocket, lyrics
  - Embeds `radio-engine` — no external C++ process, no HTTP inter-process communication
  - Static web UI in `radio-backend/static/` (Vue 3 SFC + Vite + TypeScript)
  - Admin routes split into `routes/admin/{users,stats,songs,upload,settings,playback,download,ncm,logout}.rs`
  - Shared metadata utilities in `services/metadata.rs` (reuses engine's `parse_artist_title`)
- **Legacy C++ Engine** (`legacy/cpp-engine/`) — archived, not built or used

## Build

```bash
./build_release.sh               # Full build (Rust backend + engine)
```

- Source files: `radio-engine/src/*.rs` (8 modules) + `radio-backend/src/*.rs` (10+ modules).
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

- `play_queue` (filenames) and `play_queue_metadata` (`TrackMetadata`) are parallel vectors in the radio-engine player; any mutation must be mirrored under the play_queue lock.
- All paths in `play_queue` and `PlaybackState.file_path` are **relative to `media_path`**. Use `radio_engine::util::resolve_media_path()` when ffmpeg needs an absolute path.
- `RingBuffer` capacity must be a power of two (enforced at construction).
- All core engine types (`RingBuffer`, `Player`, `StreamServer`) live in `radio-engine/src/`.
- The backend uses a `device_cookie_middleware` that auto-creates device users on first visit — every HTTP response sets or refreshes the httpOnly cookie.
- `PlaybackState.playlist_index` is the array index into `play_queue`, **not** a DB `songs.id`.
- `PlaybackStatus` is a strongly-typed enum (`Playing` / `Stopped` / `Crossfading`); serde serializes to snake_case strings for frontend compatibility.

## Known quirks

### Audio engine (radio-engine)
- The engine emits audio stream at `/stream` from the same HTTP server (port 2241), not a separate port.
- Play queue scanning uses `ffmpeg` for metadata extraction (same as legacy C++ engine).
- `init_play_queue` scans **recursively**; subdirectories under `media/` are supported.
- `TrackMetadata` fields: `duration_ms` (i64, not f64), `cover_data` (binary Vec<u8>), `embedded_lyrics` (String). These are reserved for future features (cover art extraction, embedded lyrics → .lrc conversion).

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
- **`stream_base` config** (`[audio_engine].stream_base`) supports three modes:
  - `"auto"` (default) — detects reverse proxy via `X-Forwarded-*` / `Host` headers and builds full URL dynamically.
  - Relative path (e.g. `/stream`) — resolved by frontend against `window.location.origin`.
  - Absolute URL (e.g. `http://...`) — used directly.
- **Missing cover art returns a placeholder SVG** — the `/api/songs/{id}/cover` endpoint returns a default music-note icon when no cover exists, rather than a 404 error.
- **Backend pre-parses lyrics** — LRC files are parsed into `Vec<LyricsLineDto>` in the WebSocket poller and sent as `lyrics_lines`; the frontend no longer needs `parseLyrics()`.

## No tests / no CI

- Unit tests exist only in `radio-engine/src/ring_buffer.rs` (9 tests). No backend tests.
- No lint/typecheck config, no CI workflows.
- Manual verification: build, run in `dist/`, hit API endpoints with curl or browser.
- Server logs to `dist/server.log`.
