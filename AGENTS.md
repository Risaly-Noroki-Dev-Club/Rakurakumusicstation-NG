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
  - Static web UI in `radio-backend/static/`

## Build

```bash
# Release (one-shot: builds hiredis, compiles C++ and optional Rust)
./build_release.sh

# Debug build — C++ audio engine
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 \
  -lpthread -lssl -lcrypto -I. -Ithird_party/hiredis \
  third_party/hiredis/libhiredis.a

# Build hiredis (one-time)
(cd third_party/hiredis && gcc -c -O2 -I. hiredis.c alloc.c async.c net.c read.c sds.c sockcompat.c dict.c && ar rcs libhiredis.a *.o && rm -f *.o)
```

- `crow_all.h` must exist in the repo root before building. `build_release.sh` does **not** download it.
- Only two C++ source files: `radioserver.cpp` + `metadata.cpp`.
- Link flags `-lssl -lcrypto` are required (`crow_all.h` uses them).
- C++17 is required.
- Release build adds `-O3 -flto -march=native -w`; debug build omits `-w`.

## Runtime

```bash
cd dist
./start.sh       # starts audio engine, logs to server.log
./stop.sh
```

- The C++ engine must run from inside `dist/` (or any dir containing `media/`).
- `build_release.sh` preserves `dist/media/` and `dist/playlist_order.json` across rebuilds.
- Redis is optional for the C++ engine — if unavailable it runs standalone.

## Key invariants

- `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are parallel vectors; any mutation must be mirrored under `playlist_mutex_`.
- `BroadcastBuffer` capacity must be a power of two (enforced at construction via `throw`).
- All core classes (`RadioServer`, `BroadcastBuffer`, `StreamServer`, `AudioPlayer`, `WebServer`, `RedisBridge`) live in `radioserver.cpp`.

## Important codebase gotchas

- **`Rakurakumusicstation-NG/`** is a nested directory containing its own `.git` repo — likely cruft from a migration.
- `secrets.json` and `settings.json` contain credentials. Never commit them.
- `crow_all.h` and `radioserver` (top-level) are build artifacts; delete before committing.

## No tests / no CI

- No unit tests, no lint/typecheck config, no CI workflows.
- Manual verification: build, run in `dist/`, hit API endpoints with curl or browser.
- Server logs to `dist/server.log`.
