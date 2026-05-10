# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Architecture

Single Rust binary (`radio-backend`) on port **2241** serving everything: REST API, WebSocket, audio stream (`/stream`), and static SPA files. The audio engine (`radio-engine`) is a local Rust crate embedded in-process — no separate process, no Redis.

```
radio-engine/        # Rust library crate — audio pipeline
radio-backend/       # Rust binary crate — HTTP server, auth, DB, WebSocket
radio-backend/frontend/  # Vue 3 + Vite + TypeScript SPA
dist/                # Build output (binary + static + config + media)
audio-engine/        # CMake-based C++ engine (not used in main build)
legacy/cpp-engine/   # Archived C++ engine
```

## Build

```bash
# Full release build (Rust backend + engine only; run from repo root)
./build_release.sh

# Rust backend only (debug)
cd radio-backend && cargo build

# Run a single Rust test
cd radio-engine && cargo test ring_buffer

# Frontend dev server (hot-reload, proxies API to localhost:2241)
cd radio-backend/frontend && npm run dev   # → http://localhost:5173

# Frontend production build (outputs to radio-backend/static/)
cd radio-backend/frontend && npm run build
```

## Running

```bash
cd dist
./start.sh    # nohup, writes .server.pid, logs → server.log
./stop.sh     # reads .server.pid, pgrep fallback
```

`dist/` must contain `media/` and `config.toml`. First run: edit `config.toml`, set `[device].admin_setup_token`, then place audio files in `dist/media/`.

## Source Map

### radio-engine/src/

| File | Role |
|------|------|
| `player.rs` | ffmpeg fork → MP3 pipe → ring buffer; `play_queue` + `play_queue_metadata` management; crossfade |
| `ring_buffer.rs` | Lock-free ring buffer (512 KB, power-of-2). Unit tests live here (9 tests). |
| `stream.rs` | Creates the HTTP streaming response from the ring buffer |
| `metadata.rs` | ffprobe-based metadata extraction (`TrackMetadata`) |
| `types.rs` | `PlaybackState`, `TrackMetadata`, `AudioCommand`, `PlaybackStatus` enum |
| `config.rs` | Engine constants: bitrate, sample rate, chunk size, crossfade duration |
| `util.rs` | `resolve_media_path` / `relativize_media_path` |

### radio-backend/src/

| File | Role |
|------|------|
| `main.rs` | Axum bootstrap, device cookie middleware, WS broadcaster |
| `db.rs` | SQLite pool, `AppState` (holds `Arc<Player>`, `Arc<RingBuffer>`, broadcast channel) |
| `auth.rs` | Device token cookie auth, `require_auth_from_headers()`, `AuthUser` extractor |
| `config.rs` | TOML config loading (`config.toml`) |
| `models.rs` | Serde structs: `WsMessage`, `Song`, `PlaybackState`, `AudioCommand` |
| `lyrics.rs` | LRC parser → `Vec<LyricsLineDto>` |
| `websocket.rs` | `/ws` — broadcasts from in-process state; 30s heartbeat |
| `queue_manager.rs` | Queue CRUD + rate limiting |
| `services/metadata.rs` | Shared metadata helpers (reuses engine's `parse_artist_title`) |
| `routes/mod.rs` | Router assembly + `/stream` handler + `/api/station` |
| `routes/auth.rs` | `/api/auth/*` — device token, promote via setup token |
| `routes/songs.rs` | `/api/songs/*` |
| `routes/queue.rs` | `/api/queue/*`, `/api/now-playing` |
| `routes/playlist.rs` | `/api/playlists/*` |
| `routes/favorites.rs` | `/api/favorites/*` |
| `routes/admin.rs` | `/api/admin/*` |
| `routes/ncm.rs` | `/api/ncm/*` — NetEase Cloud Music integration |

### radio-backend/frontend/src/

| File | Role |
|------|------|
| `App.vue` | Root layout: HeaderBar + NavTabs + `<router-view>` + ToastContainer |
| `store.ts` | `reactive()` singleton — all UI state; exports `formatTime`, `toast`, `applyTheme` |
| `api.ts` | HTTP fetch + WebSocket (auto-reconnect, exponential backoff) + polling fallback |
| `router.ts` | `/player`, `/queue`, `/library`, `/admin/:subtab`; admin guard |
| `types.ts` | TypeScript interfaces for all API responses and WS messages |

## Key Invariants

- `play_queue` (filenames) and `play_queue_metadata` (`TrackMetadata`) are parallel vectors; any mutation must be mirrored under the play_queue lock in `Player`.
- All paths in `play_queue` and `PlaybackState.file_path` are **relative to `media_path`**. Use `radio_engine::util::resolve_media_path()` when passing an absolute path to ffmpeg.
- `RingBuffer` capacity must be a power of two (enforced at construction).
- `PlaybackState.playlist_index` is the index into `play_queue`, **not** a DB `songs.id`.
- `PlaybackStatus` is a strongly-typed enum (`Playing` / `Stopped` / `Crossfading`); serde serializes to snake_case strings.
- Every HTTP response sets/refreshes the httpOnly `device_token` cookie via `device_cookie_middleware` — device users are auto-created on first visit.
- No auth middleware in the Axum layer — each protected handler calls `require_auth_from_headers()` or uses `AuthUser` extractor directly.
- `POST /api/admin/settings` writes `config.toml` but changes take effect only after restart.
- `stream_base` config supports `"auto"` (detects reverse proxy via headers), a relative path (e.g. `/stream`), or an absolute URL.
- `/stream` uses a **bounded** `mpsc::channel(4)` plus `send_timeout(5s)` and a 60s idle timeout in the spawned task. Reverting to `unbounded_channel` reintroduces a connection/fd/task leak: when a client disappears during a moment when hyper hasn't tried to write yet, hyper never drops the body and the task parks in `wait_for_data` forever.
- The HTTP listener is created via `bind_with_keepalive()` in `main.rs`, not `tokio::net::TcpListener::bind`. It sets SO_KEEPALIVE + TCP_KEEPIDLE/INTVL/CNT (20s/10s/3) on the listening socket so accepted connections inherit them — kernel kills dead clients in ~50s. Do not replace with stock `bind()`.
- WS `playback_state` messages include the full `lyrics_lines` array **only on song change**. Periodic 500ms updates set `lyrics_lines: None` and only carry `lyrics_line` (current line index). The frontend caches lyrics in `store.lyricsLines` and won't repopulate it from a `null` payload — broadcasting full lyrics every tick caused multi-MB/s allocation churn and ~30 MB of broadcast-channel residue.
- `extract_metadata()` intentionally leaves `embedded_lyrics` and `cover_data` empty. Nothing downstream reads them; populating `embedded_lyrics` via ffprobe at startup adds an extra subprocess fork per track (minutes for large libraries) and tens of MB of permanent memory.
- When testing locally, beware of `HTTP_PROXY=http://127.0.0.1:10808` and similar in the shell environment — services like xray will transparently relay localhost connections and hold their upstream side open after the client exits, looking exactly like a server-side fd/connection leak. Use `curl --noproxy '*' …` for any traffic-pattern measurement.

## Testing

Unit tests exist only in `radio-engine/src/ring_buffer.rs` (9 tests). No backend or frontend tests; no CI.

Manual verification: build, run in `dist/`, test API with curl or browser, watch `dist/server.log`.
