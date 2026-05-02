// =========================================================================
// Rakuraku Music Station NG — Vue 3 Frontend
// =========================================================================
const { createApp, reactive, ref, computed, watch, onMounted, onUnmounted, nextTick } = Vue;

// ─── CONSTANTS ─────────────────────────────────────────────────────────────
const BACKEND_URL = window.location.origin;
const WS_URL = (window.location.protocol === 'https:' ? 'wss://' : 'ws://') + window.location.host + '/ws';
let STREAM_URL = '/stream';
let AUDIO_ENGINE_URL = '';
const THEMES = ['auto', 'light', 'dark'];

// ─── NON-REACTIVE STATE ────────────────────────────────────────────────────
let ws = null;
let mediaSource = null;
let sourceBuffer = null;
let downloadPoller = null;
let searchTimer = null;

// ─── REACTIVE STORE ────────────────────────────────────────────────────────
const store = reactive({
    token: localStorage.getItem('radio_token') || null,
    currentUser: null,
    stationName: '电台',
    activeTab: 'player',
    activeAdminTab: 'users',
    showAuth: false,
    authMode: 'login',
    authError: '',
    authUsername: '',
    authPassword: '',
    themeIdx: (() => {
        const saved = localStorage.getItem('radio_theme') || 'auto';
        const idx = THEMES.indexOf(saved);
        return idx >= 0 ? idx : 0;
    })(),

    coverLoadError: false,
    playbackState: {
        song_id: 0, title: '', artist: '', position_ms: 0,
        duration_ms: 0, lyrics_line: null, status: 'stopped', cover_url: ''
    },
    lyricsLines: [],
    useFileMode: false,

    queue: [],
    history: [],

    searchQuery: '',
    searchResults: [],
    newPlaylistName: '',

    users: [],
    adminLogs: [],
    adminSongs: [],
    adminStats: null,

    uploadFile: null,
    uploadFileName: '',
    uploadStatus: '',
    uploadStatusType: '',

    downloadPlaylist: '',
    downloadQuality: 'exhigh',
    downloadFormat: 'mp3',
    downloadRunning: false,
    downloadStatusMsg: '',
    downloadStatusType: '',
    downloadLog: '',

    ncmBadge: '未配置',
    ncmBadgeClass: 'none',
    ncmActiveTab: 'cookie',
    ncmCookie: '',
    ncmPhone: '',
    ncmPassword: '',
    ncmResult: '',
    ncmResultType: '',

    settingsStationName: '',
    settingsSubtitle: '',
    settingsPrimaryColor: '#764ba2',
    settingsSecondaryColor: '#667eea',
    settingsBgColor: '#f4f4f9',
    settingsAdminPassword: '',
    settingsResult: '',
    settingsResultType: '',

    toasts: [],
});

// ─── UTILITIES ─────────────────────────────────────────────────────────────
function formatTime(ms) {
    if (!ms || ms < 0) return '0:00';
    const secs = Math.floor(ms / 1000);
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return m + ':' + s.toString().padStart(2, '0');
}

function escapeHtml(str) {
    if (!str) return '';
    const d = document.createElement('div');
    d.textContent = str;
    return d.innerHTML;
}

function toast(message, level) {
    level = level || 'info';
    const id = Date.now() + Math.random();
    store.toasts.push({ id, message, level });
    setTimeout(() => {
        const idx = store.toasts.findIndex(t => t.id === id);
        if (idx >= 0) store.toasts.splice(idx, 1);
    }, 4000);
}

// ─── THEME ─────────────────────────────────────────────────────────────────
function applyTheme() {
    const theme = THEMES[store.themeIdx];
    if (theme === 'auto') {
        document.documentElement.removeAttribute('data-theme');
    } else {
        document.documentElement.setAttribute('data-theme', theme);
    }
    localStorage.setItem('radio_theme', theme);
}

function cycleTheme() {
    store.themeIdx = (store.themeIdx + 1) % THEMES.length;
    applyTheme();
}

applyTheme();

// ─── API HELPER ────────────────────────────────────────────────────────────
async function api(url, opts) {
    opts = opts || {};
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

// ─── STATION INFO ──────────────────────────────────────────────────────────
async function loadStationInfo() {
    try {
        const res = await fetch(BACKEND_URL + '/api/station');
        const data = await res.json();
        if (data) {
            const info = data.data || data;
            store.stationName = info.name;
            document.title = info.name;
            document.querySelector('meta[name="theme-color"]').content = info.primary_color;
            document.documentElement.style.setProperty('--primary', info.primary_color);
            document.documentElement.style.setProperty('--secondary', info.secondary_color);
            document.documentElement.style.setProperty('--bg', info.bg_color);
            if (info.stream_url) {
                STREAM_URL = info.stream_url;
                const u = new URL(info.stream_url, window.location.origin);
                AUDIO_ENGINE_URL = u.origin;
            }
        }
    } catch (e) { /* ignore */ }
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
            } catch (e) { /* ignore malformed */ }
        };
        ws.onclose = () => { console.log('[WS] Disconnected, reconnecting in 3s...'); setTimeout(connectWebSocket, 3000); };
        ws.onerror = () => { /* ignore */ };
    } catch (e) {
        setTimeout(connectWebSocket, 3000);
    }
}

function handleWsMessage(msg) {
    switch (msg.type) {
    case 'playback_state': {
        const prevSongId = store.playbackState.song_id;
        store.playbackState.song_id = msg.song_id || 0;
        store.playbackState.title = msg.title || '';
        store.playbackState.artist = msg.artist || '';
        store.playbackState.position_ms = msg.position_ms || 0;
        store.playbackState.duration_ms = msg.duration_ms || 0;
        store.playbackState.lyrics_line = msg.lyrics_line;
        store.playbackState.status = msg.status || 'stopped';
        store.playbackState.cover_url = msg.cover_url || '';
        if (msg.song_id !== prevSongId) {
            store.coverLoadError = false;
        }
        if (msg.lyrics_text && (msg.song_id !== prevSongId || store.lyricsLines.length === 0)) {
            parseLyrics(msg.lyrics_text);
        }
        break;
    }
    case 'queue_update':
        toast((msg.requested_by || '某人') + ' 为电台点了《' + (msg.song_title || '未知歌曲') + '》', 'info');
        refreshQueue();
        break;
    case 'notice':
        toast(msg.message, msg.level === 'error' ? 'error' : 'info');
        break;
    case 'ping':
        if (ws && ws.readyState === WebSocket.OPEN) ws.send('pong');
        break;
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
    store.lyricsLines = merged;
}

// ─── POLLING ───────────────────────────────────────────────────────────────
function refreshPlaybackPoll() {
    if (ws && ws.readyState === WebSocket.OPEN) return;
    fetch(BACKEND_URL + '/api/now-playing')
        .then(r => r.json())
        .then(resp => {
            if (!resp.success || !resp.data || !resp.data.song) return;
            const d = resp.data;
            const prevId = store.playbackState.song_id;
            store.playbackState.song_id = d.song.id;
            store.playbackState.title = d.song.title;
            store.playbackState.artist = d.song.artist;
            store.playbackState.position_ms = d.position_ms || 0;
            store.playbackState.duration_ms = d.duration_ms || 0;
            store.playbackState.lyrics_line = d.lyrics_line;
            store.playbackState.cover_url = '';
            if (d.song.id !== prevId) store.coverLoadError = false;
            if (d.lyrics_text) parseLyrics(d.lyrics_text);
        }).catch(() => {});
}

// ─── QUEUE ─────────────────────────────────────────────────────────────────
async function refreshQueue() {
    try {
        const res = await fetch(BACKEND_URL + '/api/queue');
        const data = await res.json();
        if (!data.success) return;
        store.queue = data.data || [];
        await refreshHistory();
    } catch (e) { /* ignore */ }
}

async function refreshHistory() {
    try {
        const res = await fetch(BACKEND_URL + '/api/queue/history');
        const data = await res.json();
        if (data.success) store.history = (data.data || []).slice(0, 20);
    } catch (e) { /* ignore */ }
}

async function removeQueueItem(id) {
    if (!store.token) return;
    try {
        await fetch(BACKEND_URL + '/api/queue/' + id, {
            method: 'DELETE',
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        refreshQueue();
    } catch (e) { toast('移除失败', 'error'); }
}

// ─── AUTH ──────────────────────────────────────────────────────────────────
function openAuth() {
    store.authMode = 'login';
    store.authError = '';
    store.authUsername = '';
    store.authPassword = '';
    store.showAuth = true;
}

function closeAuth() {
    store.showAuth = false;
    store.authError = '';
    store.authUsername = '';
    store.authPassword = '';
}

async function doAuthFn(username, password) {
    username = (username || '').trim();
    password = password || '';

    if (username.length < 3 || password.length < 6) {
        store.authError = '用户名3-32字符，密码至少6字符';
        return;
    }

    store.authError = '';
    store.authUsername = username;
    store.authPassword = password;

    var endpoint = store.authMode === 'login' ? '/api/auth/login' : '/api/auth/register';
    try {
        var res = await fetch(BACKEND_URL + endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username: username, password: password })
        });
        var data = await res.json();
        if (data.success && data.data) {
            store.token = data.data.token;
            store.currentUser = data.data.user;
            localStorage.setItem('radio_token', store.token);
            store.showAuth = false;
            store.authError = '';
            store.authUsername = '';
            store.authPassword = '';
            toast(store.authMode === 'login' ? '登录成功' : '注册成功', 'success');
        } else {
            store.authError = (data && data.error) || '操作失败';
        }
    } catch (e) {
        store.authError = '无法连接到服务器';
    }
}

function toggleAuthMode() {
    store.authMode = store.authMode === 'login' ? 'register' : 'login';
    store.authError = '';
}

async function loadCurrentUser() {
    if (!store.token) return;
    try {
        const res = await fetch(BACKEND_URL + '/api/auth/me', {
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        const data = await res.json();
        if (data.success) {
            store.currentUser = data.data;
        } else {
            store.token = null;
            store.currentUser = null;
            localStorage.removeItem('radio_token');
        }
    } catch (e) { /* ignore */ }
}

function logout() {
    store.token = null;
    store.currentUser = null;
    localStorage.removeItem('radio_token');
    store.activeTab = 'player';
    toast('已退出登录', 'info');
}

// ─── PLAYBACK MODE ─────────────────────────────────────────────────────────
function switchPlaybackMode(audioEl) {
    store.useFileMode = !store.useFileMode;

    if (store.useFileMode) {
        if (typeof MediaSource !== 'undefined') {
            startFilePlayback(audioEl);
        } else {
            toast('你的浏览器不支持推文件模式，使用推流模式', 'error');
            store.useFileMode = false;
        }
    } else {
        audioEl.src = STREAM_URL;
        audioEl.load();
        audioEl.play().catch(() => {});
    }
}

function startFilePlayback(audio) {
    mediaSource = new MediaSource();
    audio.src = URL.createObjectURL(mediaSource);

    mediaSource.addEventListener('sourceopen', async () => {
        try {
            sourceBuffer = mediaSource.addSourceBuffer('audio/mpeg');
            fetchFileChunk(0);
        } catch (e) {
            toast('推文件模式初始化失败，回退到推流模式', 'error');
            store.useFileMode = false;
            audio.src = STREAM_URL;
            audio.load();
            audio.play().catch(() => {});
        }
    });
}

async function fetchFileChunk(offset) {
    if (store.playbackState.song_id <= 0) return;
    var chunkSize = 256 * 1024;
    try {
        const res = await fetch(AUDIO_ENGINE_URL + '/file/' + store.playbackState.song_id, {
            headers: { 'Range': 'bytes=' + offset + '-' }
        });
        if (!res.ok) return;
        const buffer = await res.arrayBuffer();
        if (buffer.byteLength > 0 && sourceBuffer && !sourceBuffer.updating) {
            sourceBuffer.appendBuffer(buffer);
        }
    } catch (e) { /* ignore */ }
}

function volumeDown(audioEl) {
    audioEl.volume = Math.max(0, audioEl.volume - 0.1);
}
function volumeUp(audioEl) {
    audioEl.volume = Math.min(1, audioEl.volume + 0.1);
}

// ─── LIBRARY / SEARCH ──────────────────────────────────────────────────────
function onSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(async () => {
        const q = store.searchQuery.trim();
        if (!q) {
            store.searchResults = [];
            return;
        }
        try {
            const res = await fetch(BACKEND_URL + '/api/songs?q=' + encodeURIComponent(q) + '&limit=50');
            const data = await res.json();
            if (data.success) {
                store.searchResults = data.data && data.data.data ? data.data.data : [];
            }
        } catch (e) { /* ignore */ }
    }, 300);
}

async function addToQueue(songId) {
    if (!store.token) { toast('请先登录再点歌', 'error'); openAuth(); return; }
    try {
        const res = await fetch(BACKEND_URL + '/api/queue', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + store.token },
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

function addToMyPlaylist(songId) {
    toast('请先在"我的歌单"中创建歌单，再从歌单中添加', 'info');
}

// ─── PLAYLISTS ─────────────────────────────────────────────────────────────
async function createPlaylist() {
    if (!store.token || !store.newPlaylistName.trim()) return;
    try {
        const res = await fetch(BACKEND_URL + '/api/playlists', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + store.token },
            body: JSON.stringify({ name: store.newPlaylistName.trim() })
        });
        const data = await res.json();
        if (data.success) {
            toast('歌单创建成功', 'success');
            store.newPlaylistName = '';
        }
    } catch (e) { /* ignore */ }
}

// ─── ADMIN: USERS ──────────────────────────────────────────────────────────
async function loadAdminUsersAndLogs() {
    if (!store.currentUser || store.currentUser.role !== 'admin') return;
    try {
        const [usersRes, logsRes] = await Promise.all([
            fetch(BACKEND_URL + '/api/admin/users', { headers: { 'Authorization': 'Bearer ' + store.token } }),
            fetch(BACKEND_URL + '/api/admin/logs', { headers: { 'Authorization': 'Bearer ' + store.token } }),
        ]);
        store.users = ((await usersRes.json()).data || []);
        store.adminLogs = ((await logsRes.json()).data || []);
    } catch (e) { /* ignore */ }
}

async function adminBan(userId) {
    try {
        await fetch(BACKEND_URL + '/api/admin/users/' + userId + '/ban', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        loadAdminUsersAndLogs();
    } catch (e) { /* ignore */ }
}

async function adminUnban(userId) {
    try {
        await fetch(BACKEND_URL + '/api/admin/users/' + userId + '/unban', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        loadAdminUsersAndLogs();
    } catch (e) { /* ignore */ }
}

// ─── ADMIN: SONGS ──────────────────────────────────────────────────────────
async function loadAdminSongs() {
    if (!store.currentUser || store.currentUser.role !== 'admin') return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/songs', {
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        store.adminSongs = ((await res.json()).data || []);
    } catch (e) { toast('加载歌曲列表失败', 'error'); }
}

async function adminDeleteSong(id) {
    if (!confirm('确定要删除这首歌曲吗？此操作不可撤销。')) return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/songs/' + id, {
            method: 'DELETE',
            headers: { 'Authorization': 'Bearer ' + store.token }
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
            headers: { 'Authorization': 'Bearer ' + store.token }
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
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        const data = await res.json();
        toast(data.data || '已切到下一首', 'success');
    } catch (e) { toast('操作失败', 'error'); }
}

async function adminPlayPrev() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/playlist/prev', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        const data = await res.json();
        toast(data.data || '已切到上一首', 'success');
    } catch (e) { toast('操作失败', 'error'); }
}

// ─── ADMIN: STATS ──────────────────────────────────────────────────────────
async function loadAdminStats() {
    if (!store.currentUser || store.currentUser.role !== 'admin') return;
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/stats', {
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        store.adminStats = (await res.json()).data || {};
    } catch (e) { /* ignore */ }
}

// ─── ADMIN: UPLOAD ─────────────────────────────────────────────────────────
function handleFileSelect(event) {
    const file = event.target.files[0];
    if (file) {
        store.uploadFile = file;
        store.uploadFileName = file.name;
        store.uploadStatus = '';
        store.uploadStatusType = '';
    }
}

async function uploadSong() {
    if (!store.uploadFile) {
        store.uploadStatus = '请选择文件';
        store.uploadStatusType = 'error';
        return;
    }
    if (store.uploadFile.size > 100 * 1024 * 1024) {
        store.uploadStatus = '文件大小超过 100MB 限制';
        store.uploadStatusType = 'error';
        return;
    }

    const formData = new FormData();
    formData.append('file', store.uploadFile);
    store.uploadStatus = '上传中...';
    store.uploadStatusType = 'info';

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/upload', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + store.token },
            body: formData
        });
        const data = await res.json();
        if (data.success) {
            store.uploadStatus = '✅ ' + data.data;
            store.uploadStatusType = 'success';
            store.uploadFile = null;
            store.uploadFileName = '';
        } else {
            store.uploadStatus = '❌ ' + (data.error || '上传失败');
            store.uploadStatusType = 'error';
        }
    } catch (e) {
        store.uploadStatus = '❌ 上传失败';
        store.uploadStatusType = 'error';
    }
}

// ─── ADMIN: DOWNLOAD ───────────────────────────────────────────────────────
async function startDownload() {
    const playlist = store.downloadPlaylist.trim();
    if (!playlist) {
        store.downloadStatusMsg = '请输入歌单内容';
        store.downloadStatusType = 'error';
        return;
    }

    store.downloadRunning = true;
    store.downloadLog = '';
    store.downloadStatusMsg = '正在提交任务...';
    store.downloadStatusType = 'info';

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/download', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + store.token
            },
            body: JSON.stringify({
                playlist: playlist,
                quality: store.downloadQuality,
                format: store.downloadFormat
            })
        });
        const data = await res.json();
        if (data.success) {
            store.downloadStatusMsg = '下载中，请稍候...';
            store.downloadStatusType = 'info';
            pollDownloadStatus();
        } else {
            store.downloadStatusMsg = '❌ ' + (data.error || '启动失败');
            store.downloadStatusType = 'error';
            store.downloadRunning = false;
        }
    } catch (e) {
        store.downloadStatusMsg = '❌ 启动失败';
        store.downloadStatusType = 'error';
        store.downloadRunning = false;
    }
}

function pollDownloadStatus() {
    if (downloadPoller) clearInterval(downloadPoller);
    downloadPoller = setInterval(async () => {
        try {
            const res = await fetch(BACKEND_URL + '/api/admin/download/status', {
                headers: { 'Authorization': 'Bearer ' + store.token }
            });
            if (!res.ok) return;
            const data = await res.json();
            if (!data.success) return;
            const status = data.data;
            if (status.log) store.downloadLog = status.log;
            if (!status.running) {
                clearInterval(downloadPoller);
                downloadPoller = null;
                store.downloadRunning = false;
                store.downloadStatusMsg = '✅ 下载完成';
                store.downloadStatusType = 'success';
            }
        } catch (e) { /* ignore */ }
    }, 2000);
}

// Check ongoing download on init
(function () {
    if (!store.token) return;
    fetch(BACKEND_URL + '/api/admin/download/status', {
        headers: { 'Authorization': 'Bearer ' + store.token }
    }).then(r => r.json()).then(data => {
        if (data.success && data.data && data.data.running) {
            store.downloadRunning = true;
            store.downloadLog = data.data.log || '';
            store.downloadStatusMsg = '下载中，请稍候...';
            store.downloadStatusType = 'info';
            pollDownloadStatus();
        }
    }).catch(() => {});
})();

// ─── ADMIN: NCM ────────────────────────────────────────────────────────────
async function loadNcmStatus() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm', {
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        if (!res.ok) return;
        const d = await res.json();
        if (!d.success) return;
        const data = d.data;
        if (data.configured) {
            const label = data.method === 'cookie' ? 'Cookie 已配置' : '手机号 ' + (data.phone_hint || '') + ' 已配置';
            store.ncmBadge = '✓ ' + label;
            store.ncmBadgeClass = 'ok';
        } else {
            store.ncmBadge = '未配置（游客模式）';
            store.ncmBadgeClass = 'none';
        }
    } catch (e) { /* ignore */ }
}

async function saveNcmSettings() {
    const payload = store.ncmActiveTab === 'cookie'
        ? { cookie: store.ncmCookie.trim(), phone: '', password: '' }
        : { phone: store.ncmPhone.trim(), password: store.ncmPassword, cookie: '' };

    if (store.ncmActiveTab === 'cookie' && !payload.cookie)
        return showNcmResult('请填写 Cookie', 'error');
    if (store.ncmActiveTab === 'phone' && (!payload.phone || !payload.password))
        return showNcmResult('请填写手机号和密码', 'error');

    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + store.token
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
    showNcmResult('测试中...', 'info');
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/ncm/test', {
            method: 'POST',
            headers: { 'Authorization': 'Bearer ' + store.token }
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
}

function showNcmResult(msg, type) {
    store.ncmResult = msg;
    store.ncmResultType = type;
}

// ─── ADMIN: SETTINGS ───────────────────────────────────────────────────────
async function loadSettings() {
    try {
        const res = await fetch(BACKEND_URL + '/api/admin/settings', {
            headers: { 'Authorization': 'Bearer ' + store.token }
        });
        if (!res.ok) {
            store.settingsResult = '加载设置失败';
            store.settingsResultType = 'error';
            return;
        }
        const data = await res.json();
        if (!data.success) {
            store.settingsResult = data.error || '加载失败';
            store.settingsResultType = 'error';
            return;
        }
        const s = data.data;
        store.settingsStationName = s.station_name || 'Rakuraku Music Station';
        store.settingsSubtitle = s.subtitle || '';
        store.settingsPrimaryColor = s.primary_color || '#764ba2';
        store.settingsSecondaryColor = s.secondary_color || '#667eea';
        store.settingsBgColor = s.bg_color || '#f4f4f9';
        store.settingsAdminPassword = '';
        store.settingsResult = '设置已加载';
        store.settingsResultType = 'info';
    } catch (error) {
        store.settingsResult = '加载设置失败: ' + error.message;
        store.settingsResultType = 'error';
    }
}

async function saveSettings() {
    try {
        const settings = {
            station_name: store.settingsStationName.trim(),
            subtitle: store.settingsSubtitle.trim(),
            primary_color: store.settingsPrimaryColor,
            secondary_color: store.settingsSecondaryColor,
            bg_color: store.settingsBgColor,
        };
        if (store.settingsAdminPassword.trim()) {
            settings.admin_password = store.settingsAdminPassword.trim();
        }

        const res = await fetch(BACKEND_URL + '/api/admin/settings', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': 'Bearer ' + store.token
            },
            body: JSON.stringify(settings)
        });

        const data = await res.json();
        if (data.success) {
            store.settingsResult = data.data || '设置保存成功';
            store.settingsResultType = 'success';
        } else {
            store.settingsResult = '保存失败: ' + (data.error || '');
            store.settingsResultType = 'error';
        }
    } catch (error) {
        store.settingsResult = '保存失败: ' + error.message;
        store.settingsResultType = 'error';
    }
}

// ─── TAB SWITCHING ─────────────────────────────────────────────────────────
function switchTab(name) {
    store.activeTab = name;
    if (name === 'queue') refreshQueue();
    if (name === 'admin') {
        loadAdminUsersAndLogs();
        loadAdminStats();
    }
}

function switchAdminTab(name) {
    store.activeAdminTab = name;
    if (name === 'users') loadAdminUsersAndLogs();
    if (name === 'songs') loadAdminSongs();
    if (name === 'stats') loadAdminStats();
    if (name === 'settings') loadSettings();
    if (name === 'ncm') loadNcmStatus();
}

// ─── VUE APP ───────────────────────────────────────────────────────────────
const app = createApp({
    setup() {
        const audioEl = ref(null);
        const authUsernameEl = ref(null);
        const authPasswordEl = ref(null);
        const coverContainer = ref(null);

        const themeIcon = computed(() => {
            const icons = { auto: '🌓', light: '☀️', dark: '🌙' };
            return icons[THEMES[store.themeIdx]] || '🌓';
        });

        const showCover = computed(() => {
            return !!(store.playbackState.cover_url || store.playbackState.song_id > 0);
        });

        const coverSrc = computed(() => {
            if (store.playbackState.cover_url) return store.playbackState.cover_url;
            if (store.playbackState.song_id > 0) return BACKEND_URL + '/api/songs/' + store.playbackState.song_id + '/cover';
            return '';
        });

        const titleDisplay = computed(() => {
            return store.playbackState.title || '等待播放...';
        });

        const artistDisplay = computed(() => {
            return store.playbackState.artist || '';
        });

        const progressPct = computed(() => {
            return store.playbackState.duration_ms > 0
                ? Math.min(100, (store.playbackState.position_ms / store.playbackState.duration_ms) * 100)
                : 0;
        });

        const currentTimeFormatted = computed(() => {
            return formatTime(store.playbackState.position_ms);
        });

        const totalTimeFormatted = computed(() => {
            return formatTime(store.playbackState.duration_ms);
        });

        const lyricActiveIdx = computed(() => {
            if (store.lyricsLines.length === 0) return -1;
            const pos = store.playbackState.position_ms;
            let activeIdx = -1;
            for (let i = store.lyricsLines.length - 1; i >= 0; i--) {
                if (store.lyricsLines[i].timeMs <= pos) {
                    activeIdx = i;
                    break;
                }
            }
            return activeIdx;
        });

        let lastScrolledIdx = -1;
        function scrollLyricIntoView(el) {
            if (lyricActiveIdx.value >= 0 && lyricActiveIdx.value !== lastScrolledIdx) {
                lastScrolledIdx = lyricActiveIdx.value;
                nextTick(function () {
                    if (el && el.scrollIntoView) {
                        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
                    }
                });
            }
        }

        function onSwitchPlaybackMode() {
            if (audioEl.value) switchPlaybackMode(audioEl.value);
        }

        function onVolumeDown() {
            if (audioEl.value) volumeDown(audioEl.value);
        }
        function onVolumeUp() {
            if (audioEl.value) volumeUp(audioEl.value);
        }

        function doAuth() {
            var u = authUsernameEl.value ? authUsernameEl.value.value : '';
            var p = authPasswordEl.value ? authPasswordEl.value.value : '';
            doAuthFn(u, p);
        }

        return {
            store,
            themeIcon,
            showCover,
            coverSrc,
            titleDisplay,
            artistDisplay,
            progressPct,
            currentTimeFormatted,
            totalTimeFormatted,
            lyricActiveIdx,
            scrollLyricIntoView,
            audioEl,
            authUsernameEl,
            authPasswordEl,
            switchPlaybackMode: onSwitchPlaybackMode,
            volumeDown: onVolumeDown,
            volumeUp: onVolumeUp,

            // Methods exposed to template
            cycleTheme,
            switchTab,
            switchAdminTab,
            formatTime,
            openAuth,
            closeAuth,
            doAuth,
            toggleAuthMode,
            logout,
            onSearchInput,
            addToQueue,
            addToMyPlaylist,
            createPlaylist,
            removeQueueItem,

            // Admin methods
            adminBan,
            adminUnban,
            adminDeleteSong,
            adminRescanSongs,
            adminPlayNext,
            adminPlayPrev,
            handleFileSelect,
            uploadSong,
            startDownload,
            saveNcmSettings,
            testNcmLogin,
            loadSettings,
            saveSettings,
        };
    },

    mounted() {
        loadStationInfo();
        connectWebSocket();
        if (store.token) {
            loadCurrentUser();
        }
        setInterval(refreshQueue, 5000);
        setInterval(refreshPlaybackPoll, 2000);
    },

    beforeUnmount() {
        if (downloadPoller) clearInterval(downloadPoller);
        if (searchTimer) clearTimeout(searchTimer);
        if (ws) ws.close();
    }
});

app.mount('#app');

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
