# AGENTS.md

## Shape

- Main app is a single Rust binary: `radio-backend` serves REST, WebSocket `/ws`, audio stream `/stream`, and static files on port `2241`.
- `radio-engine` is a local Rust library embedded by `radio-backend`; do not reintroduce a separate audio process, Redis, or HTTP IPC for engine control.
- `radio-backend/frontend` is Vue 3 + Vite + TypeScript; `npm run build` writes production assets to `radio-backend/static/`, which the backend serves via `ServeDir::new("static")` fallback.
- `legacy/cpp-engine/`, `audio-engine/`, root `music_dl.py`, and `deploy/Makefile` are legacy/deploy leftovers and are not part of the normal root build.

## Commands

- Full release package from repo root: `./build_release.sh`. It runs `cargo build --release` in `radio-backend`, copies the binary to `dist/`, copies existing `radio-backend/static/`, seeds `dist/config.toml` only if missing, and preserves `dist/media/`.
- Backend-only debug build: `cd radio-backend && cargo build`.
- Engine tests: `cd radio-engine && cargo test ring_buffer`. These are the only checked-in Rust unit tests.
- Frontend dev server: `cd radio-backend/frontend && npm run dev`; Vite listens on `5173` and proxies `/api`, `/ws`, and `/stream` to `localhost:2241`.
- Frontend production check/build: `cd radio-backend/frontend && npm run build`; this runs `vue-tsc -b && vite build` and updates `radio-backend/static/`.
- Runtime after release build: `cd dist && ./start.sh` and `cd dist && ./stop.sh`; logs go to `dist/server.log` and lifecycle uses `dist/.server.pid`.

## Runtime Config

- The backend loads `config.toml` from the current working directory unless `RADIO_CONFIG` is set, so normal runtime is from inside `dist/`.
- Required runtime tools are `ffmpeg` for playback and `ffprobe` for duration/tag scanning.
- SQLite is the only supported DB path in practice; migrations are embedded with `sqlx::migrate!("./migrations")` and run on startup.
- `RADIO_DATABASE_URL`, `RADIO_SERVER_PORT`, `RADIO_LOG_LEVEL`, `RADIO_MEDIA_PATH`, `RADIO_STREAM_BASE`, `RADIO_STATION_NAME`, `RADIO_ADMIN_SETUP_TOKEN`, `RADIO_NCM_DEVICE_ID`, and `RADIO_NCM_DOWNLOAD_CONCURRENCY` override TOML values.
- Settings saved through `/api/admin/settings` write `config.toml` but are not hot-reloaded; restart the server.

## Engine Invariants

- Paths stored in `play_queue`, request tracks, and `PlaybackState.file_path` are relative to `media_path`; use `radio_engine::util::{resolve_media_path, relativize_media_path}` at filesystem/ffmpeg boundaries.
- `Player.play_queue` and `Player.play_queue_metadata` are parallel vectors; mutate them together under the queue lock.
- `PlaybackState.playlist_index` is a `play_queue` index, not a `songs.id`; requested tracks may have `playlist_index == -1`, so song-change logic should key on `file_path` or DB song id as appropriate.
- `RingBuffer` capacity must be a nonzero power of two.
- `/stream` uses a bounded channel (`STREAM_CHANNEL_CAPACITY = 4`) so dead clients surface via backpressure; do not switch it to an unbounded channel.
- `main.rs` binds with `bind_with_keepalive()` rather than `tokio::net::TcpListener::bind`; this is intentional to clean up dead streaming connections.
- `extract_metadata()` intentionally leaves `embedded_lyrics` and `cover_data` empty to avoid startup subprocess and memory blowups.

## Backend/Frontend Contracts

- Device auth is cookie-based. `device_cookie_middleware` creates/refreshes httpOnly `device_token`; protected handlers explicitly call `require_device_auth()` / `require_admin_auth()` or use `AuthUser` helpers, not Axum auth middleware.
- `/api/now-playing` is DB-oriented and not the source for smooth real-time position; WebSocket playback messages are the realtime path.
- WebSocket `playback_state` sends full `lyrics_lines` only once per song change; later 500ms updates send `lyrics_line` and `lyrics_lines: null`. The frontend caches lyrics in `store.lyricsLines`.
- `stream_base = "auto"` derives `/stream` URLs from `Host` / `X-Forwarded-*`; it can also be a relative path like `/stream` or an absolute URL.
- Uploads/downloads/rescans that add media should send `AudioCommandType::ReloadQueue` so the embedded engine sees new files without a server restart.

## Local Verification Gotchas

- For traffic or leak tests against localhost, bypass shell proxy variables with `curl --noproxy '*' ...`; local proxies can make closed clients look like server-side fd leaks.
- If frontend code changed, run `npm run build` before `./build_release.sh`; the root script does not install npm deps or invoke Vite.
