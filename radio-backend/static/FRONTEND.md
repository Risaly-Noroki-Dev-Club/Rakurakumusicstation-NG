# Rakuraku Music Station NG — Frontend Design Spec

> For frontend developers joining this project. Covers the browser-side Web UI architecture, color system, API conventions, and development workflow.

---

## File Locations

| File | Use | Status |
|------|-----|--------|
| `radio-backend/static/index.html` | Main player UI (HTML structure) | Active |
| `radio-backend/static/style.css` | All CSS styles | Active |
| `radio-backend/static/app.js` | All JavaScript logic | Active |
| `radio-backend/static/manifest.json` | PWA Web Manifest | Active |
| `radio-backend/static/sw.js` | PWA Service Worker | Active |

Legacy C++-served templates (`index.html`, `panel.html`, `login.html`) have been removed. The Rust backend now exclusively serves the web UI.

---

## Color System

### Rule

**Never hardcode a hex color in CSS properties or inline styles.** All colors use CSS custom properties (variables), defined centrally in `:root`.

### Available Variables

```css
:root {
    /* Injected by Rust backend /api/station at runtime */
    --primary: #764ba2;
    --secondary: #667eea;
    --bg: #f4f4f9;

    /* Semantic tokens maintained by frontend */
    --card: #ffffff;
    --text: #333;
    --text-muted: #6c757d;
    --text-inverse: #ffffff;
    --border: #e9ecef;
    --success: #28a745;
    --danger: #dc3545;
    --warning: #ffc107;
    --info: #17a2b8;

    --radius: 12px;
    --shadow: 0 4px 20px rgba(0,0,0,0.08);
}
```

### Dark Mode

Three-mode toggle (🌓 top-right corner), persisted to `localStorage.radio_theme`:

| Mode | Behavior |
|------|----------|
| `auto` (default) | Follows OS `prefers-color-scheme` |
| `light` | Force light (`:root[data-theme="light"]`) |
| `dark` | Force dark (`:root[data-theme="dark"]`) |

When adding new variables, provide defaults for all three modes.

---

## Backend Communication

### Port Convention

| Service | Port | Notes |
|---------|------|-------|
| Rust Backend (API + WebSocket + static files) | 2241 | Frontend loads from here |
| C++ Audio Engine (audio stream) | 2240 | Headless — no web UI |

### URL Detection

```javascript
const BACKEND_URL = window.location.origin;
const AUDIO_ENGINE_URL = protocol + '//' + hostname + ':2240';
const WS_URL = (protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/ws';
```

All API requests use absolute URLs (`BACKEND_URL + '/api/...'`). Do not hardcode ports.

### WebSocket Messages

JSON with `type` field:

```json
{"type":"playback_state","song_id":1,"title":"...","artist":"...","position_ms":120000,"duration_ms":240000,"lyrics_line":5,"lyrics_text":"[00:00.00]...\n[00:10.00]...","status":"playing","stream_url":"http://.../stream","file_url":"http://.../file/1"}
{"type":"queue_update","action":"added","song_title":"...","requested_by":"...","queue_size":5}
{"type":"notice","message":"...","level":"info"}
{"type":"ping","timestamp":123456789}
```

`lyrics_line` and `lyrics_text` are pre-computed server-side. Frontend only renders.

### HTTP API (Rust Backend, port 2241)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/api/station` | — | Station name, theme colors (sets `--primary`/`--secondary`/`--bg`) |
| `GET` | `/api/now-playing` | — | Current track + lyrics (HTTP polling fallback) |
| `GET` | `/api/songs?q=&limit=20` | — | Song library search |
| `GET` | `/api/songs/{id}/cover` | — | Cover art binary |
| `GET` | `/api/queue` | — | Public queue |
| `GET` | `/api/queue/history` | — | Play history |
| `POST` | `/api/queue` | JWT | Request a song |
| `POST` | `/api/auth/register` | — | Register |
| `POST` | `/api/auth/login` | — | Login → JWT |
| `GET` | `/api/auth/me` | JWT | Current user |
| `GET` | `/api/favorites` | JWT | Favorites list |
| `POST` `DELETE` | `/api/favorites/{id}` | JWT | Add/remove favorite |

Auth: register/login returns JWT. Store in `localStorage.radio_token`. Send as `Authorization: Bearer <token>`.

---

## Global State

```javascript
let playbackState = {
    song_id: 0,
    title: '',
    artist: '',
    position_ms: 0,
    duration_ms: 0,
    lyrics_line: null,
    status: 'stopped',
    cover_url: ''
};
```

Updated via:
1. **WebSocket** (primary): `handleWsMessage` → update state → refresh UI
2. **HTTP polling** (fallback, 2s interval): `GET /api/now-playing`

When adding fields, update both paths.

---

## Development Workflow

```bash
cd radio-backend
cp config.toml.example config.toml
cargo run
# Open http://localhost:2241

# C++ engine (required for audio streaming)
cd .. && cd dist && ./start.sh

# Redis (optional — enables real-time state sync)
redis-server
```

- No build tools (no webpack/vite/npm)
- CSS in `style.css`, JS in `app.js`, HTML in `index.html`
- Static files served from disk — refresh browser after editing
- No external CDN dependencies (PWA must work offline)

---

## Admin Panel

Admin users (JWT with `role: "admin"`) get sub-tabs:

- **Users** — ban/unban, audit logs
- **Songs** — list, delete, rescan media, skip controls
- **Upload** — multipart form (≤100 MB)
- **Download** — batch download via `music_dl.py` with live log
- **NCM** — NetEase Cloud Music credentials (cookie or phone + login test)
- **Settings** — station name, theme colors, admin password
- **Stats** — user/song/queue/playlist counts
