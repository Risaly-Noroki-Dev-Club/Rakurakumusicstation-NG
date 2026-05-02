// =========================================================================
// Rakuraku Music Station NG — Frontend Application
// =========================================================================

// ─── CONFIGURATION ─────────────────────────────────────────────────────────
const BACKEND_URL = window.location.origin;
const AUDIO_ENGINE_URL = window.location.protocol + '//' + window.location.hostname + ':2240';
const WS_URL = (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.host + '/ws';
const STREAM_URL = AUDIO_ENGINE_URL + '/stream';

// ─── GLOBAL STATE ──────────────────────────────────────────────────────────
let token = localStorage.getItem('radio_token') || null;
let currentUser = null;
let ws = null;
let playbackState = { song_id: 0, title: '', artist: '', position_ms: 0, duration_ms: 0, lyrics_line: null, status: 'stopped', cover_url: '' };
let lyricsLines = [];
let useFileMode = false;
let mediaSource = null;
let sourceBuffer = null;
let fileModeQueue = [];
let activeTab = 'player';
let activeAdminTab = 'users';

// ─── THEME ─────────────────────────────────────────────────────────────────
const THEMES = ['auto', 'light', 'dark'];
let themeIdx = THEMES.indexOf(localStorage.getItem('radio_theme') || 'auto');
if (themeIdx < 0) themeIdx = 0;

function applyTheme() {
    const theme = THEMES[themeIdx];
    const icons = { auto: '🌓', light: '☀️', dark: '🌙' };
    document.getElementById('themeToggle').textContent = icons[theme];
    if (theme === 'auto') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', theme);
    }
    localStorage.setItem('radio_theme', theme);
}

function cycleTheme() {
    themeIdx = (themeIdx + 1) % THEMES.length;
    applyTheme();
}

applyTheme();

// ─── FETCH UTILITY ─────────────────────────────────────────────────────────
async function api(url, opts = {}) {
    try {
        const res = await fetch(url, opts);
        if (!res.ok && res.status >= 500) {
            toast('服务器错误 ' + res.status, 'error');
        }
        return res;
    } catch (e) {
        toast('网络不可用', 'error');
        throw e;
    }
}

// ─── INITIALIZATION ────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', async () => {
    await loadStationInfo();
    connectWebSocket();
    if (token) {
        await loadCurrentUser();
    }
    setInterval(refreshQueue, 5000);
    setInterval(refreshPlaybackPoll, 2000);
});

async function loadStationInfo() {
    try {
        const res = await fetch(BACKEND_URL + '/api/station');
        const data = await res.json();
        if (data.success && data.data) {
            document.getElementById('stationName').textContent = data.name;
            document.title = data.name;
            document.querySelector('meta[name="theme-color"]').content = data.primary_color;
            document.documentElement.style.setProperty('--primary', data.primary_color);
            document.documentElement.style.setProperty('--secondary', data.secondary_color);
            document.documentElement.style.setProperty('--bg', data.bg_color);
        }
    } catch (e) {}
}

// ─── WEBSOCKET ─────────────────────────────────────────────────────────────
function connectWebSocket() {
    try {
        ws = new WebSocket(WS_URL);
        ws.onopen = () => { console.log('[WS] Connected'); toast('已连接到电台服务器', 'success'); };
        ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                handleWsMessage(msg);
            } catch (e) {}
        };
        ws.onclose = () => { console.log('[WS] Disconnected, reconnecting in 3s...'); setTimeout(connectWebSocket, 3000); };
        ws.onerror = () => {};
    } catch (e) {
        setTimeout(connectWebSocket, 3000);
    }
}

function handleWsMessage(msg) {
    switch (msg.type) {
        case 'playback_state':
            const prevSongId = playbackState.song_id;
            playbackState = {
                song_id: msg.song_id || 0,
                title: msg.title || '',
                artist: msg.artist || '',
                position_ms: msg.position_ms || 0,
                duration_ms: msg.duration_ms || 0,
                lyrics_line: msg.lyrics_line,
                status: msg.status || 'stopped',
            };
            updateNowPlayingDisplay();
            updateProgressBar();
            if (msg.song_id) loadCoverFromWs(msg);
            if (msg.lyrics_text && (msg.song_id !== prevSongId || lyricsLines.length === 0)) {
                parseLyrics(msg.lyrics_text);
            }
            updateLyricsHighlight();
            break;

        case 'queue_update':
            toast(`${msg.requested_by || '某人'} 为电台点了《${msg.song_title || '未知歌曲'}》`, 'info');
            refreshQueue();
            break;

        case 'notice':
            toast(msg.message, msg.level === 'error' ? 'error' : 'info');
            break;

        case 'ping':
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send('pong');
            }
            break;
    }
}

// ─── NOW PLAYING DISPLAY ───────────────────────────────────────────────────
function updateNowPlayingDisplay() {
    const info = document.getElementById('nowPlayingInfo');
    if (playbackState.title) {
        info.innerHTML = `<div class="title">${escapeHtml(playbackState.title)}</div>
                          <div class="artist">${escapeHtml(playbackState.artist || '')}</div>`;
    }
    document.getElementById('totalTime').textContent = formatTime(playbackState.duration_ms);
}

function loadCover(songId) {
    const container = document.getElementById('coverContainer');
    const img = document.getElementById('coverImage');
    if (!songId || songId <= 0) {
        container.classList.remove('show');
        return;
    }
    img.src = BACKEND_URL + '/api/songs/' + songId + '/cover';
    container.classList.add('show');
}

function loadCoverFromWs(msg) {
    const container = document.getElementById('coverContainer');
    const img = document.getElementById('coverImage');
    if (msg.cover_url) {
        img.src = msg.cover_url;
        container.classList.add('show');
    } else if (msg.song_id && msg.song_id > 0) {
        loadCover(msg.song_id);
    } else {
        container.classList.remove('show');
    }
}

function updateProgressBar() {
    const pct = playbackState.duration_ms > 0
        ? Math.min(100, (playbackState.position_ms / playbackState.duration_ms) * 100)
        : 0;
    document.getElementById('progressFill').style.width = pct + '%';
    document.getElementById('currentTime').textContent = formatTime(playbackState.position_ms);
}

function refreshPlaybackPoll() {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        fetch(BACKEND_URL + '/api/now-playing')
            .then(r => r.json())
            .then(resp => {
                if (resp.success && resp.data) {
                    const d = resp.data;
                    if (d.song) {
                        playbackState.song_id = d.song.id;
                        playbackState.title = d.song.title;
                        playbackState.artist = d.song.artist;
                        playbackState.position_ms = d.position_ms || 0;
                        playbackState.duration_ms = d.duration_ms || 0;
                        playbackState.lyrics_line = d.lyrics_line;
                        updateNowPlayingDisplay();
                        updateProgressBar();
                        if (d.lyrics_text) parseLyrics(d.lyrics_text);
                        loadCover(d.song.id);
                        document.getElementById('lyricsCard').classList.toggle('hidden', !d.lyrics_text);
                    }
                }
            }).catch(() => {});
    }
}

// ─── LYRICS ────────────────────────────────────────────────────────────────
function parseLyrics(lrcText) {
    const lines = [];
    const re = /\[(\d{1,3}):(\d{1,2})(?:\.(\d{1,3}))?\](.*)/g;
    let match;
    while ((match = re.exec(lrcText)) !== null) {
        const min = parseInt(match[1]);
        const sec = parseInt(match[2]);
        const ms = match[3] ? parseInt(match[3].padEnd(3, '0')) : 0;
        const timeMs = min * 60000 + sec * 1000 + ms;
        const text = match[4].trim();
        if (text) lines.push({ timeMs, text });
    }
    lines.sort((a, b) => a.timeMs - b.timeMs);
    const merged = [];
    for (const l of lines) {
        if (merged.length > 0 && merged[merged.length - 1].timeMs === l.timeMs) {
            merged[merged.length - 1].text = l.text;
        } else {
            merged.push(l);
        }
    }
    lyricsLines = merged;
    renderLyrics();
}

function renderLyrics() {
    const box = document.getElementById('lyricsBox');
    const card = document.getElementById('lyricsCard');
    if (lyricsLines.length === 0) {
        box.innerHTML = '<div class="line inactive">暂无歌词</div>';
        return;
    }
    card.classList.remove('hidden');
    box.innerHTML = lyricsLines.map((l, i) =>
        `<div class="line inactive" data-idx="${i}">${escapeHtml(l.text)}</div>`
    ).join('');
    updateLyricsHighlight();
}

function updateLyricsHighlight() {
    if (lyricsLines.length === 0) return;
    const pos = playbackState.position_ms;
    let activeIdx = -1;
    for (let i = lyricsLines.length - 1; i >= 0; i--) {
        if (lyricsLines[i].timeMs <= pos) {
            activeIdx = i;
            break;
        }
    }
    const lines = document.querySelectorAll('#lyricsBox .line');
    lines.forEach((el, i) => {
        el.className = 'line ' + (i === activeIdx ? 'active' : 'inactive');
    });
    if (activeIdx >= 0 && lines[activeIdx]) {
        lines[activeIdx].scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
}

// ─── PLAYBACK MODE ─────────────────────────────────────────────────────────
function switchPlaybackMode() {
    useFileMode = !useFileMode;
    const audio = document.getElementById('audioPlayer');
    const badge = document.getElementById('modeBadge');

    if (useFileMode) {
        if (typeof MediaSource !== 'undefined') {
            badge.textContent = '推文件模式';
            badge.className = 'mode-badge mode-file';
            badge.classList.remove('hidden');
            startFilePlayback();
        } else {
            toast('你的浏览器不支持推文件模式，使用推流模式', 'error');
            useFileMode = false;
        }
    } else {
        badge.classList.add('hidden');
        audio.src = STREAM_URL;
        audio.load();
        audio.play().catch(() => {});
    }
}

function startFilePlayback() {
    const audio = document.getElementById('audioPlayer');
    mediaSource = new MediaSource();
    audio.src = URL.createObjectURL(mediaSource);

    mediaSource.addEventListener('sourceopen', async () => {
        try {
            sourceBuffer = mediaSource.addSourceBuffer('audio/mpeg');
            fetchFileChunk(0);
        } catch (e) {
            toast('推文件模式初始化失败，回退到推流模式', 'error');
            useFileMode = false;
            document.getElementById('modeBadge').classList.add('hidden');
            audio.src = STREAM_URL;
            audio.load();
            audio.play().catch(() => {});
        }
    });
}

async function fetchFileChunk(offset) {
    if (playbackState.song_id <= 0) return;
    const chunkSize = 256 * 1024;
    const rangeEnd = Math.min(offset + chunkSize - 1, (playbackState.duration_ms / 1000) * 16 * 1024);
    try {
        const res = await fetch(AUDIO_ENGINE_URL + '/file/' + playbackState.song_id, {
            headers: { 'Range': `bytes=${offset}-` }
        });
        if (!res.ok) return;
        const buffer = await res.arrayBuffer();
        if (buffer.byteLength > 0 && sourceBuffer && !sourceBuffer.updating) {
            sourceBuffer.appendBuffer(buffer);
        }
    } catch (e) {}
}

function volumeDown() {
    const a = document.getElementById('audioPlayer');
    a.volume = Math.max(0, a.volume - 0.1);
}
function volumeUp() {
    const a = document.getElementById('audioPlayer');
    a.volume = Math.min(1, a.volume + 0.1);
}

// ─── QUEUE ─────────────────────────────────────────────────────────────────
async function refreshQueue() {
    try {
        const res = await fetch(BACKEND_URL + '/api/queue');
        const data = await res.json();
        if (!data.success) return;

        const items = data.data || [];
        document.getElementById('queueCount').textContent = `(${items.length}首)`;
        const list = document.getElementById('queueList');

        if (items.length === 0) {
            list.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">队列为空</div>';
            return;
        }

        list.innerHTML = items.map(item => {
            const song = item.song || {};
            const isPlaying = item.status === 'playing';
            return `<div class="queue-item">
                <div class="info">
                    <span>${escapeHtml(song.title || '未知歌曲')}</span>
                    ${song.artist ? ` <span style="color:var(--text-muted);font-size:0.85em">- ${escapeHtml(song.artist)}</span>` : ''}
                    <div class="meta">
                        <span class="badge ${isPlaying ? 'badge-playing' : 'badge-pending'}">${isPlaying ? '播放中' : '等待'}</span>
                        点歌: ${escapeHtml(item.requested_by)}
                        ${currentUser && currentUser.role === 'admin' && !isPlaying ? `<button class="btn btn-danger btn-small" style="margin-left:8px" onclick="removeQueueItem(${item.id})">移除</button>` : ''}
                    </div>
                </div>
            </div>`;
        }).join('');

        refreshHistory();
    } catch (e) {}
}

async function refreshHistory() {
    try {
        const res = await fetch(BACKEND_URL + '/api/queue/history');
        const data = await res.json();
        if (!data.success) return;
        const items = data.data || [];
        const list = document.getElementById('historyList');
        if (items.length === 0) {
            list.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">暂无播放历史</div>';
            return;
        }
        list.innerHTML = items.slice(0, 20).map(h => {
            const song = h.song || {};
            return `<div class="queue-item">
                <span>${escapeHtml(song.title || '未知')}</span>
                <span style="color:var(--text-muted);font-size:0.8em">${h.requested_by || ''} · ${h.played_at || ''}</span>
            </div>`;
        }).join('');
    } catch (e) {}
}

async function removeQueueItem(id) {
    if (!token) return;
    try {
        await fetch(`${BACKEND_URL}/api/queue/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        refreshQueue();
    } catch (e) { toast('移除失败', 'error'); }
}

// ─── SONG LIBRARY ──────────────────────────────────────────────────────────
let searchTimer = null;
function searchSongs() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(async () => {
        const q = document.getElementById('searchInput').value.trim();
        if (!q) {
            document.getElementById('songList').innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">输入关键词搜索曲库</div>';
            return;
        }
        try {
            const res = await fetch(`${BACKEND_URL}/api/songs?q=${encodeURIComponent(q)}&limit=50`);
            const data = await res.json();
            if (!data.success) return;
            const songs = data.data?.data || [];
            renderSongList(songs);
        } catch (e) {}
    }, 300);
}

function renderSongList(songs) {
    const list = document.getElementById('songList');
    if (songs.length === 0) {
        list.innerHTML = '<div style="text-align:center;color:var(--text-muted);padding:20px">未找到匹配的歌曲</div>';
        return;
    }
    list.innerHTML = songs.map(s => `
        <div class="song-item">
            <div>
                <div style="font-weight:600">${escapeHtml(s.title)}</div>
                <div style="font-size:0.85em;color:var(--text-muted)">${escapeHtml(s.artist)} · ${escapeHtml(s.album)} · ${formatTime(s.duration_ms)}</div>
            </div>
            <div style="display:flex;gap:4px">
                <button class="btn btn-primary btn-small" onclick="addToQueue(${s.id})" title="投喂到电台">📻 点歌</button>
                ${currentUser ? `<button class="btn btn-secondary btn-small" onclick="addToMyPlaylist(${s.id})" title="收藏到歌单">⭐</button>` : ''}
            </div>
        </div>
    `).join('');
}

async function addToQueue(songId) {
    if (!token) { toast('请先登录再点歌', 'error'); showAuthView(); return; }
    try {
        const res = await fetch(BACKEND_URL + '/api/queue', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
            body: JSON.stringify({ song_id: songId })
        });
        const data = await res.json();
        if (data.success) {
            toast('已添加到电台队列！', 'success');
            refreshQueue();
        } else {
            toast(data.error || '点歌失败', 'error');
        }
    } catch (e) { toast('请求失败', 'error'); }
}

async function addToMyPlaylist(songId) {
    toast('请先在"我的歌单"中创建歌单，再从歌单中添加', 'info');
}

// ─── AUTH ──────────────────────────────────────────────────────────────────
function showAuthView() {
    document.getElementById('authModal').classList.remove('hidden');
    document.getElementById('authError').style.display = 'none';
}
function hideAuthView() {
    document.getElementById('authModal').classList.add('hidden');
}
let authMode = 'login';
function toggleAuthMode() {
    authMode = authMode === 'login' ? 'register' : 'login';
    document.getElementById('authTitle').textContent = authMode === 'login' ? '登录' : '注册';
    document.getElementById('authSubmitBtn').textContent = authMode === 'login' ? '登录' : '注册';
    document.querySelector('#authModal .btn-secondary').textContent = authMode === 'login' ? '切换到注册' : '切换到登录';
}

async function doAuth() {
    const username = document.getElementById('authUsername').value.trim();
    const password = document.getElementById('authPassword').value;
    const errEl = document.getElementById('authError');

    if (username.length < 3 || password.length < 6) {
        errEl.textContent = '用户名3-32字符，密码至少6字符';
        errEl.style.display = 'block';
        return;
    }

    const endpoint = authMode === 'login' ? '/api/auth/login' : '/api/auth/register';
    try {
        const res = await fetch(BACKEND_URL + endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });
        const data = await res.json();
        if (data.success) {
            token = data.data.token;
            currentUser = data.data.user;
            localStorage.setItem('radio_token', token);
            updateUserUI();
            hideAuthView();
            toast(authMode === 'login' ? '登录成功' : '注册成功', 'success');
        } else {
            errEl.textContent = data.error || '操作失败';
            errEl.style.display = 'block';
        }
    } catch (e) {
        errEl.textContent = '无法连接到服务器';
        errEl.style.display = 'block';
    }
}

async function loadCurrentUser() {
    try {
        const res = await fetch(BACKEND_URL + '/api/auth/me', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        if (data.success) {
            currentUser = data.data;
            updateUserUI();
        } else {
            token = null;
            localStorage.removeItem('radio_token');
        }
    } catch (e) {}
}

function updateUserUI() {
    if (currentUser) {
        document.getElementById('userDisplay').textContent = currentUser.username;
        document.getElementById('userDisplay').classList.remove('hidden');
        document.getElementById('loginBtn').classList.add('hidden');
        document.getElementById('logoutBtn').classList.remove('hidden');
        document.getElementById('myPlaylistsCard').classList.remove('hidden');

        if (currentUser.role === 'admin') {
            document.getElementById('adminTab').classList.remove('hidden');
        }
    }
}

function logout() {
    token = null;
    currentUser = null;
    localStorage.removeItem('radio_token');
    document.getElementById('userDisplay').classList.add('hidden');
    document.getElementById('loginBtn').classList.remove('hidden');
    document.getElementById('logoutBtn').classList.add('hidden');
    document.getElementById('adminTab').classList.add('hidden');
    document.getElementById('myPlaylistsCard').classList.add('hidden');
    toast('已退出登录', 'info');
}

// ─── ADMIN ─────────────────────────────────────────────────────────────────

// Admin sub-tab switching
function switchAdminTab(name) {
    activeAdminTab = name;
    document.querySelectorAll('.subtab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('.admin-subpanel').forEach(p => p.classList.remove('active'));

    // Find the clicked subtab button and activate it
    const subtabs = document.querySelectorAll('.subtab');
    const texts = { users: '用户管理', songs: '歌曲管理', upload: '上传', download: '下载', ncm: '网易云', settings: '设置', stats: '统计' };
    subtabs.forEach(t => {
        if (t.textContent.includes(texts[name] || '')) t.classList.add('active');
    });

    const panel = document.getElementById('admin-panel-' + name);
    if (panel) panel.classList.add('active');

    // Load data for the selected panel
    if (name === 'users') loadAdminUsersAndLogs();
    if (name === 'songs') loadAdminSongs();
    if (name === 'stats') loadAdminStats();
    if (name === 'settings') loadSettings();
    if (name === 'ncm') loadNcmStatus();
}

// User management (existing admin features)
async function loadAdminUsersAndLogs() {
    if (!currentUser || currentUser.role !== 'admin') return;
    try {
        const [usersRes, logsRes] = await Promise.all([
            fetch(BACKEND_URL + '/api/admin/users', { headers: { 'Authorization': 'Bearer ' + token } }),
            fetch(BACKEND_URL + '/api/admin/logs', { headers: { 'Authorization': 'Bearer ' + token } }),
        ]);
        const users = (await usersRes.json()).data || [];
        const logs = (await logsRes.json()).data || [];

        document.getElementById('usersTable').innerHTML = users.map(u =>
            `<tr>
                <td>${u.id}</td><td>${escapeHtml(u.username)}</td><td>${u.role}</td>
                <td>${u.is_banned ? '🔴已封禁' : '🟢正常'}</td>
                <td>
                    ${u.is_banned
                        ? `<button class="btn btn-secondary btn-small" onclick="adminUnban(${u.id})">解封</button>`
                        : `<button class="btn btn-danger btn-small" onclick="adminBan(${u.id})">封禁</button>`}
                </td>
            </tr>`
        ).join('');

        document.getElementById('adminLogs').innerHTML = logs.map(l =>
            `<div style="padding:4px 0;border-bottom:1px solid rgba(0,0,0,0.05)">
                <span style="color:var(--text-muted)">[${l.created_at}]</span>
                ${escapeHtml(l.action)} — ${escapeHtml(l.details)}
            </div>`
        ).join('');
    } catch (e) {}
}

async function adminBan(userId) {
    try {
        await fetch(`${BACKEND_URL}/api/admin/users/${userId}/ban`, {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        loadAdminUsersAndLogs();
    } catch (e) {}
}
async function adminUnban(userId) {
    try {
        await fetch(`${BACKEND_URL}/api/admin/users/${userId}/unban`, {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        loadAdminUsersAndLogs();
    } catch (e) {}
}

// Stats
async function loadAdminStats() {
    if (!currentUser || currentUser.role !== 'admin') return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/stats', { headers: { 'Authorization': 'Bearer ' + token } });
        const stats = (await res.json()).data || {};
        document.getElementById('adminStats').innerHTML = `
            👥 用户: ${stats.users || 0} &nbsp;|&nbsp;
            🎵 歌曲: ${stats.songs || 0} &nbsp;|&nbsp;
            📋 队列: ${stats.queue_size || 0} &nbsp;|&nbsp;
            📁 歌单: ${stats.playlists || 0}
        `;
    } catch (e) {}
}

// ─── ADMIN: Song Management ────────────────────────────────────────────────
async function loadAdminSongs() {
    if (!currentUser || currentUser.role !== 'admin') return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/songs', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        const songs = data.data || [];
        const table = document.getElementById('adminSongsTable');

        if (songs.length === 0) {
            table.innerHTML = '<tr><td colspan="3" style="text-align:center;color:var(--text-muted);padding:20px">暂无歌曲，请点击"重新扫描"</td></tr>';
            return;
        }

        table.innerHTML = songs.map(s => `
            <tr>
                <td class="song-title" title="${escapeHtml(s.title)}">${escapeHtml(s.title)}</td>
                <td style="color:var(--text-muted)">${escapeHtml(s.artist || '-')}</td>
                <td class="actions">
                    <button class="btn btn-danger btn-small" onclick="adminDeleteSong(${s.id})" title="删除">🗑️</button>
                </td>
            </tr>
        `).join('');
    } catch (e) { toast('加载歌曲列表失败', 'error'); }
}

async function adminDeleteSong(id) {
    if (!confirm('确定要删除这首歌曲吗？此操作不可撤销。')) return;
    try {
        const res = await fetch(`${BACKEND_URL}/api/admin/songs/${id}`, {
            method: 'DELETE',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        if (data.success) {
            toast(data.data, 'success');
            loadAdminSongs();
        } else {
            toast(data.error || '删除失败', 'error');
        }
    } catch (e) { toast('删除失败', 'error'); }
}

async function adminRescanSongs() {
    try {
        toast('正在扫描媒体目录...', 'info');
        const res = await fetch(BACKEND_URL + '/api/admin/rescan-songs', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        if (data.success) {
            toast(data.data, 'success');
            loadAdminSongs();
        } else {
            toast(data.error || '扫描失败', 'error');
        }
    } catch (e) { toast('扫描失败', 'error'); }
}

async function adminPlayNext() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/playlist/next', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        toast(data.data || '已切到下一首', 'success');
    } catch (e) { toast('操作失败', 'error'); }
}

async function adminPlayPrev() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/playlist/prev', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        toast(data.data || '已切到上一首', 'success');
    } catch (e) { toast('操作失败', 'error'); }
}

// ─── ADMIN: Upload ─────────────────────────────────────────────────────────
let selectedFile = null;

function handleFileSelect(event) {
    selectedFile = event.target.files[0];
    document.getElementById('uploadBtn').disabled = !selectedFile;
}

async function uploadSong() {
    if (!selectedFile) {
        showUploadStatus('请选择文件', 'error');
        return;
    }
    if (selectedFile.size > 100 * 1024 * 1024) {
        showUploadStatus('文件大小超过 100MB 限制', 'error');
        return;
    }

    const formData = new FormData();
    formData.append('file', selectedFile);
    showUploadStatus('上传中...', 'info');
    document.getElementById('uploadBtn').disabled = true;

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/upload', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token },
            body: formData
        });
        const data = await res.json();
        if (data.success) {
            showUploadStatus('✅ ' + data.data, 'success');
            document.getElementById('uploadFileInput').value = '';
            selectedFile = null;
        } else {
            showUploadStatus('❌ ' + (data.error || '上传失败'), 'error');
            document.getElementById('uploadBtn').disabled = false;
        }
    } catch (e) {
        showUploadStatus('❌ 上传失败', 'error');
        document.getElementById('uploadBtn').disabled = false;
    }
}

function showUploadStatus(msg, type) {
    const el = document.getElementById('uploadStatus');
    el.textContent = msg;
    el.className = 'upload-status ' + type;
    el.style.display = msg ? 'block' : 'none';
    if (type !== 'info') setTimeout(() => { el.style.display = 'none'; }, 5000);
}

// ─── ADMIN: Download ───────────────────────────────────────────────────────
let downloadPoller = null;

function showDownloadStatus(msg, type) {
    const el = document.getElementById('downloadStatusMsg');
    el.textContent = msg;
    el.className = 'upload-status ' + type;
    el.style.display = msg ? 'block' : 'none';
}

async function startDownload() {
    const playlist = document.getElementById('downloadPlaylistInput').value.trim();
    if (!playlist) {
        showDownloadStatus('请输入歌单内容', 'error');
        return;
    }

    const quality = document.getElementById('qualitySelect').value;
    const format = document.getElementById('formatSelect').value;

    document.getElementById('startDownloadBtn').disabled = true;
    document.getElementById('downloadLog').style.display = 'block';
    document.getElementById('downloadLog').textContent = '';
    showDownloadStatus('正在提交任务...', 'info');

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/download', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + token
            },
            body: JSON.stringify({ playlist, quality, format })
        });
        const data = await res.json();
        if (data.success) {
            showDownloadStatus('下载中，请稍候...', 'info');
            pollDownloadStatus();
        } else {
            showDownloadStatus('❌ ' + (data.error || '启动失败'), 'error');
            document.getElementById('startDownloadBtn').disabled = false;
        }
    } catch (e) {
        showDownloadStatus('❌ 启动失败', 'error');
        document.getElementById('startDownloadBtn').disabled = false;
    }
}

function pollDownloadStatus() {
    if (downloadPoller) clearInterval(downloadPoller);
    downloadPoller = setInterval(async () => {
        try {
            const res = await fetch(BACKEND_URL + '/api/admin/download/status', {
                headers: { 'Authorization': 'Bearer ' + token }
            });
            if (!res.ok) return;
            const data = await res.json();
            if (!data.success) return;
            const status = data.data;
            const logEl = document.getElementById('downloadLog');
            if (status.log) {
                logEl.textContent = status.log;
                logEl.scrollTop = logEl.scrollHeight;
            }
            if (!status.running) {
                clearInterval(downloadPoller);
                downloadPoller = null;
                document.getElementById('startDownloadBtn').disabled = false;
                showDownloadStatus('✅ 下载完成', 'success');
            }
        } catch (e) {}
    }, 2000);
}

// Check for ongoing download on page load
(async () => {
    if (!token) return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/download/status', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        if (!res.ok) return;
        const data = await res.json();
        if (data.success && data.data && data.data.running) {
            document.getElementById('startDownloadBtn').disabled = true;
            document.getElementById('downloadLog').style.display = 'block';
            document.getElementById('downloadLog').textContent = data.data.log || '';
            showDownloadStatus('下载中，请稍候...', 'info');
            pollDownloadStatus();
        }
    } catch (e) {}
})();

// ─── ADMIN: NCM Settings ───────────────────────────────────────────────────
let ncmActiveTab = 'cookie';

function switchNcmTab(tab) {
    ncmActiveTab = tab;
    document.querySelectorAll('.ncm-tab').forEach(b => b.classList.remove('active'));
    const buttons = document.querySelectorAll('.ncm-tab');
    buttons.forEach(b => {
        if ((tab === 'cookie' && b.textContent.includes('Cookie')) ||
            (tab === 'phone' && b.textContent.includes('手机号'))) {
            b.classList.add('active');
        }
    });
    document.getElementById('ncmPanelCookie').classList.toggle('active', tab === 'cookie');
    document.getElementById('ncmPanelPhone').classList.toggle('active', tab === 'phone');
}

async function loadNcmStatus() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        if (!res.ok) return;
        const d = await res.json();
        if (!d.success) return;
        const badge = document.getElementById('ncmBadge');
        const data = d.data;
        if (data.configured) {
            const label = data.method === 'cookie' ? 'Cookie 已配置' : `手机号 ${data.phone_hint || ''} 已配置`;
            badge.textContent = '✓ ' + label;
            badge.className = 'ncm-badge ok';
        } else {
            badge.textContent = '未配置（游客模式）';
            badge.className = 'ncm-badge none';
        }
    } catch (e) {}
}

async function saveNcmSettings() {
    const payload = ncmActiveTab === 'cookie'
        ? { cookie: document.getElementById('ncmCookie').value.trim(), phone: '', password: '' }
        : { phone: document.getElementById('ncmPhone').value.trim(),
            password: document.getElementById('ncmPassword').value,
            cookie: '' };

    if (ncmActiveTab === 'cookie' && !payload.cookie)
        return showNcmResult('请填写 Cookie', 'error');
    if (ncmActiveTab === 'phone' && (!payload.phone || !payload.password))
        return showNcmResult('请填写手机号和密码', 'error');

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + token
            },
            body: JSON.stringify(payload)
        });
        const data = await res.json();
        if (data.success) {
            showNcmResult('✅ 保存成功', 'success');
            loadNcmStatus();
        } else {
            showNcmResult('❌ ' + (data.error || '保存失败'), 'error');
        }
    } catch (e) { showNcmResult('❌ 请求失败', 'error'); }
}

async function testNcmLogin() {
    const btn = document.getElementById('ncmTestBtn');
    btn.disabled = true;
    showNcmResult('测试中...', 'info');
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm/test', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + token }
        });
        const data = await res.json();
        if (data.success) {
            const d = data.data;
            showNcmResult(
                (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败')),
                d.success ? 'success' : 'error'
            );
        } else {
            showNcmResult('❌ 请求失败', 'error');
        }
    } catch (e) { showNcmResult('❌ 请求失败', 'error'); }
    btn.disabled = false;
}

function showNcmResult(msg, type) {
    const el = document.getElementById('ncmResult');
    el.textContent = msg;
    el.className = 'upload-status ' + type;
    el.style.display = 'block';
}

// ─── ADMIN: System Settings ────────────────────────────────────────────────
async function loadSettings() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/settings', {
            headers: { 'Authorization': 'Bearer ' + token }
        });
        if (!res.ok) {
            showSettingsMessage('加载设置失败', 'error');
            return;
        }
        const data = await res.json();
        if (!data.success) {
            showSettingsMessage(data.error || '加载失败', 'error');
            return;
        }
        const s = data.data;
        document.getElementById('settingStationName').value = s.station_name || 'Rakuraku Music Station';
        document.getElementById('settingSubtitle').value = s.subtitle || '';
        document.getElementById('settingPrimaryColor').value = s.primary_color || '#764ba2';
        document.getElementById('settingSecondaryColor').value = s.secondary_color || '#667eea';
        document.getElementById('settingBgColor').value = s.bg_color || '#f4f4f9';
        document.getElementById('settingAdminPassword').value = '';
        showSettingsMessage('设置已加载', 'info');
    } catch (error) {
        showSettingsMessage('加载设置失败: ' + error.message, 'error');
    }
}

async function saveSettings() {
    try {
        const settings = {
            station_name: document.getElementById('settingStationName').value.trim(),
            subtitle: document.getElementById('settingSubtitle').value.trim(),
            primary_color: document.getElementById('settingPrimaryColor').value,
            secondary_color: document.getElementById('settingSecondaryColor').value,
            bg_color: document.getElementById('settingBgColor').value,
        };

        const password = document.getElementById('settingAdminPassword').value.trim();
        if (password) {
            settings.admin_password = password;
        }

        const res = await fetch(BACKEND_URL + '/api/admin/settings', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + token
            },
            body: JSON.stringify(settings)
        });

        const data = await res.json();
        if (data.success) {
            showSettingsMessage(data.data || '设置保存成功', 'success');
        } else {
            showSettingsMessage('保存失败: ' + (data.error || ''), 'error');
        }
    } catch (error) {
        showSettingsMessage('保存失败: ' + error.message, 'error');
    }
}

function showSettingsMessage(message, type) {
    const el = document.getElementById('settingsResult');
    el.textContent = message;
    el.className = 'result-message ' + type;
    el.style.display = 'block';
    if (type === 'success') {
        setTimeout(() => el.style.display = 'none', 5000);
    }
}

// ─── TAB SWITCHING ─────────────────────────────────────────────────────────
function switchTab(name) {
    activeTab = name;
    document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
    document.querySelectorAll('[id^="tab-"]').forEach(d => d.classList.add('hidden'));

    const tabs = document.querySelectorAll('.tab');
    const indexes = { player: 0, queue: 1, library: 2, admin: 3 };
    if (tabs[indexes[name]]) tabs[indexes[name]].classList.add('active');

    const tabContent = document.getElementById('tab-' + name);
    if (tabContent) tabContent.classList.remove('hidden');

    if (name === 'queue') refreshQueue();
    if (name === 'admin') {
        loadAdminUsersAndLogs();
        loadAdminStats();
    }
}

// ─── UTILITY ───────────────────────────────────────────────────────────────
function formatTime(ms) {
    if (!ms || ms < 0) return '0:00';
    const secs = Math.floor(ms / 1000);
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m + ':' + s.toString().padStart(2, '0');
}

function escapeHtml(str) {
    const d = document.createElement('div');
    d.textContent = str;
    return d.innerHTML;
}

function toast(message, level = 'info') {
    const container = document.getElementById('toastContainer');
    const el = document.createElement('div');
    el.className = 'toast toast-' + level;
    el.textContent = message;
    container.appendChild(el);
    setTimeout(() => el.remove(), 4000);
}

// ─── MY PLAYLISTS ──────────────────────────────────────────────────────────
async function createPlaylist() {
    if (!token) return;
    const name = document.getElementById('newPlaylistName').value.trim();
    if (!name) return;
    try {
        const res = await fetch(BACKEND_URL + '/api/playlists', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
            body: JSON.stringify({ name })
        });
        const data = await res.json();
        if (data.success) {
            toast('歌单创建成功', 'success');
            document.getElementById('newPlaylistName').value = '';
        }
    } catch (e) {}
}

// ─── Service Worker ────────────────────────────────────────────────────────
if ('serviceWorker' in navigator) {
    window.addEventListener('load', async () => {
        try {
            await navigator.serviceWorker.register('/sw.js');
            console.log('Service Worker 注册成功');
        } catch (error) {
            console.error('Service Worker 注册失败:', error);
        }
    });
}
