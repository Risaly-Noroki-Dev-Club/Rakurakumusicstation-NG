# AGENTS.md

This file provides guidance for OpenCode sessions working in this repository.

## Build

```bash
# Release (one-shot: installs deps, fetches crow_all.h, builds dist/)
./build_release.sh

# Debug build — two compile units
g++ radioserver.cpp metadata.cpp -o radioserver -std=c++17 -g -O0 -lpthread -lssl -lcrypto -I.
```

- `crow_all.h` is downloaded by `build_release.sh` and skipped if already present. Delete it to pull latest.
- Only two source files exist: `radioserver.cpp` + `metadata.cpp`. The debug build must include both.
- Link flags `-lssl -lcrypto` are required (session IDs use `RAND_bytes`, password compare uses `CRYPTO_memcmp`).
- C++17 is required.

## Runtime

```bash
cd dist          # server uses relative paths from CWD
./start.sh       # sets zh_CN.UTF-8 locale, backgrounds server, logs to server.log
./stop.sh
```

- The server binary must run from inside `dist/` (or any dir containing `media/` and `settings.json`).
- `build_release.sh` preserves `dist/media/` and `dist/settings.json` across rebuilds.
- Templates are embedded into `embedded_templates.hpp` during build. At runtime they are overridden by files found in `./templates/` or the CWD (lookup order: CWD → `templates/` → embedded).

## Important codebase gotchas

- **`.gitignore` has an unresolved merge conflict.** Lines 1-58 have `<<<<<<< HEAD` / `=======` / `>>>>>>>` markers. Do not trust it until resolved.
- **`CLAUDE.md` is listed in `.gitignore`** (line 37 of the HEAD side), so it won't be committed.
- **`authmiddleware.hpp` does not exist** on disk despite being referenced in README and CLAUDE.md.
- **`STREAM_PORT` and `separate_stream_port` appear only in docs**, not in the actual source code. CLAUDE.md port-mode notes are unverified against code.
- **`Rakurakumusicstation-NG/`** is a nested directory containing its own `.git` repo — likely cruft from a migration.

## Key invariants

- `playlist_` (filenames) and `playlist_metadata_` (TrackMetadata) are parallel vectors; any mutation to one must be mirrored on the other under `playlist_mutex_`.
- `BroadcastBuffer` capacity must be a power of two (enforced at construction via `throw`).
- All core classes (`RadioServer`, `BroadcastBuffer`, `StreamServer`, `AudioPlayer`, `WebServer`) live in `radioserver.cpp`.

## No tests / no CI

- No unit tests, no lint/typecheck config, no CI workflows.
- Manual verification: build, run in `dist/`, hit API endpoints with curl or browser.
- Server logs to `dist/server.log`.
