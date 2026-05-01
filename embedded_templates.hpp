#pragma once
#include <string>
#include <unordered_map>
namespace EmbeddedTemplates {
static const std::unordered_map<std::string, std::string> templates = {
  {"index.html", R"RKTML(
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>{{STATION_NAME}}</title>

    <!-- PWA相关配置 -->
    <meta name="theme-color" content="{{PRIMARY_COLOR}}">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="{{STATION_NAME}}">
    <link rel="manifest" href="/manifest.json">
    <link rel="icon" href="/favicon.ico" type="image/x-icon">
    <link rel="apple-touch-icon" href="/favicon.ico">
    <style>
        /* 接收后端传来的自定义颜色变量 */
        :root {
            --primary-color: {{PRIMARY_COLOR}};
            --secondary-color: {{SECONDARY_COLOR}};
            --bg-color: {{BG_COLOR}};
        }
        
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            background: var(--bg-color);
            min-height: 100vh;
            padding: 20px;
            color: #333;
        }
        .container {
            max-width: 1000px;
            margin: 0 auto;
            background: rgba(255, 255, 255, 0.95);
            border-radius: 20px;
            padding: 30px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            backdrop-filter: blur(10px);
        }
        header { text-align: center; margin-bottom: 40px; }
        h1 {
            font-size: 3em;
            color: var(--primary-color);
            margin-bottom: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 15px;
        }
        h1 .icon { font-size: 1.2em; }
        .subtitle {
            color: #666;
            font-size: 1.2em;
            margin-bottom: 30px;
        }
        .player-section, .playlist-section {
            background: #f8f9fa;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            border: 1px solid #e9ecef;
        }
        .player-section h2, .playlist-section h2 {
            color: #495057;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        audio { width: 100%; margin-bottom: 20px; border-radius: 10px; }
        .controls {
            display: flex; gap: 15px; align-items: center; flex-wrap: wrap;
        }
        button {
            background: var(--primary-color);
            color: white;
            border: none;
            padding: 12px 25px;
            border-radius: 8px;
            cursor: pointer;
            font-size: 1em;
            font-weight: 600;
            transition: all 0.3s ease;
            display: flex;
            align-items: center;
            gap: 8px;
        }
        button:hover { transform: translateY(-2px); opacity: 0.9; }
        button:active { transform: translateY(0); }
        #currentTrack {
            font-size: 1.2em; color: #495057; font-weight: 600;
            background: white; padding: 10px 20px; border-radius: 8px;
            flex-grow: 1; border: 2px solid #e9ecef;
        }
        #playlist {
            display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 12px; max-height: 400px; overflow-y: auto; padding-right: 10px;
        }
        .track {
            background: white; padding: 15px; border-radius: 8px;
            cursor: pointer; transition: all 0.2s ease; border-left: 4px solid transparent;
        }
        .track:hover { background: #e9ecef; transform: translateX(5px); }
        .track.current {
            background: rgba(102, 126, 234, 0.1);
            border-left-color: var(--secondary-color);
        }
        .track-number {
            display: inline-block; width: 25px; text-align: center;
            background: var(--secondary-color); color: white;
            border-radius: 4px; margin-right: 10px; font-size: 0.9em; padding: 2px 5px;
        }
        .stats { display: flex; gap: 20px; margin-top: 30px; flex-wrap: wrap; }
        .stat-box {
            background: white; padding: 20px; border-radius: 10px;
            flex: 1; min-width: 200px; border: 1px solid #e9ecef; text-align: center;
        }
        .stat-value { font-size: 2.5em; font-weight: bold; color: var(--secondary-color); margin: 10px 0; }
        .stat-label { color: #6c757d; font-size: 0.9em; }
        footer {
            text-align: center; margin-top: 40px; color: #6c757d;
            font-size: 0.9em; padding-top: 20px; border-top: 1px solid #e9ecef;
        }
        .admin-link {
            display: inline-block; margin-top: 10px;
            color: var(--primary-color); text-decoration: none;
            font-weight: 600; padding: 6px 14px; border-radius: 6px;
            border: 1px solid var(--primary-color); transition: all 0.2s ease;
        }
        .admin-link:hover { background: var(--primary-color); color: white; }
    /* 移动端响应式适配 */
        @media (max-width: 768px) {
            .container {
                padding: 20px 15px;
                border-radius: 15px;
            }

            h1 {
                font-size: 2.2em;
                flex-direction: column;
                gap: 10px;
            }

            .player-section, .playlist-section {
                padding: 20px 15px;
            }

            .controls {
                flex-direction: column;
                align-items: stretch;
                gap: 10px;
            }

            button {
                width: 100%;
                justify-content: center;
                padding: 14px 20px;
            }

            #playlist {
                grid-template-columns: 1fr;
                max-height: 500px;
            }

            .stats {
                flex-direction: column;
                gap: 15px;
            }

            .stat-box {
                min-width: 100%;
            }

            #currentTrack {
                text-align: center;
                padding: 12px 15px;
            }
        }

        @media (max-width: 480px) {
            .container {
                padding: 15px 10px;
            }

            h1 {
                font-size: 1.8em;
            }

            .subtitle {
                font-size: 1em;
            }

            .player-section h2, .playlist-section h2 {
                font-size: 1.2em;
            }

            audio {
                height: 40px;
            }

            .track {
                padding: 12px;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1><span class="icon">🎵</span> {{STATION_NAME}}</h1>
            <div class="subtitle">{{SUBTITLE}}</div>
        </header>
        
        <section class="player-section">
            <h2><span class="icon">▶️</span> 当前播放</h2>
            <audio id="player" controls autoplay>
                <source src="/stream" type="audio/mpeg">
            </audio>
            <div class="controls">
                <button id="nextBtn" onclick="playNext()">⏭️ 下一首</button>
                <div id="currentTrack">正在加载播放列表...</div>
            </div>
        </section>
        
        <section class="playlist-section">
            <h2><span class="icon">📋</span> 播放列表 (<span id="trackCount">0</span> 首歌曲)</h2>
            <div id="playlist">加载中...</div>
        </section>
        
        <div class="stats">
            <div class="stat-box"><div class="stat-label">在线听众</div><div class="stat-value" id="clientCount">0</div></div>
            <div class="stat-box"><div class="stat-label">总曲目</div><div class="stat-value" id="totalTracks">0</div></div>
            <div class="stat-box"><div class="stat-label">当前曲目</div><div class="stat-value" id="currentIndex">-</div></div>
        </div>
        
        <footer>
            <p>© {{STATION_NAME}} | 端口: 2240</p>
            <a class="admin-link" href="/admin">🔐 管理员入口</a>
        </footer>
    </div>

    <script>
        const allowGuestSkip = {{ALLOW_GUEST_SKIP}};

        document.addEventListener("DOMContentLoaded", () => {
            if (!allowGuestSkip) {
                document.getElementById('nextBtn').style.display = 'none';
            }
        });

        async function loadPlaylist() {
            try {
                const response = await fetch('/api/playlist');
                const data = await response.json();
                const playlist = data.playlist || [];
                const metadata = data.metadata || [];
                const currentIndex = data.current || 0;

                let currentTrackIndex = playlist.length > 0 ? currentIndex % playlist.length : 0;
                let totalTracks = playlist.length;

                const currentMeta = metadata[currentTrackIndex];
                const currentDisplay = currentMeta
                    ? (currentMeta.artist ? `${currentMeta.artist} - ${currentMeta.title}` : currentMeta.title)
                    : (playlist[currentTrackIndex] || '');

                document.getElementById('currentTrack').textContent = playlist.length > 0 ? `正在播放: ${currentDisplay}` : '播放列表为空';
                document.getElementById('trackCount').textContent = totalTracks;
                document.getElementById('totalTracks').textContent = totalTracks;
                document.getElementById('currentIndex').textContent = playlist.length > 0 ? currentTrackIndex + 1 : '-';

                let html = '';
                if (playlist.length === 0) {
                    html = '<div style="text-align: center; padding: 40px; color: #6c757d;"><div>播放列表为空，请上传音乐文件</div></div>';
                } else {
                    const tracks = metadata.length > 0 ? metadata : playlist.map(f => ({ title: f, artist: '' }));
                    tracks.forEach((track, index) => {
                        const isCurrent = index === currentTrackIndex;
                        const title = track.title || track.filename || playlist[index];
                        const artist = track.artist || '';
                        const displayText = artist ? `${artist} - ${title}` : title;
                        const clickHandler = allowGuestSkip ? `onclick="playTrack(${index})"` : '';
                        html += `
                            <div class="track ${isCurrent ? 'current' : ''}" ${clickHandler} style="${allowGuestSkip ? 'cursor:pointer' : ''}">
                                <span class="track-number">${index + 1}</span>
                                ${displayText}
                                ${isCurrent ? ' <span>▶️</span>' : ''}
                            </div>
                        `;
                    });
                }
                document.getElementById('playlist').innerHTML = html;
            } catch (error) {
                console.error('加载播放列表失败:', error);
            }
        }

        async function loadStats() {
            try {
                const response = await fetch('/api/stats');
                const data = await response.json();
                document.getElementById('clientCount').textContent = data.clients || 0;
            } catch (error) {}
        }

        async function playTrack(index) {
            if (!allowGuestSkip) return;
            try {
                await fetch('/api/play/' + index, { method: 'POST' });
                setTimeout(() => { loadPlaylist(); document.getElementById('player').load(); }, 500);
            } catch (error) { console.error(error); }
        }

        async function playNext() {
            try {
                await fetch('/api/next', { method: 'POST' });
                setTimeout(() => { loadPlaylist(); document.getElementById('player').load(); }, 500);
            } catch (error) { console.error(error); }
        }

        loadPlaylist(); loadStats();
        setInterval(loadPlaylist, 3000); setInterval(loadStats, 2000);

        // Service Worker 注册
        if ('serviceWorker' in navigator) {
            navigator.serviceWorker.register('/sw.js').catch(() => {});
        }
    </script>
</body>
</html>

)RKTML"},
  {"panel.html", R"RKTML(
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>{{STATION_NAME}} - 管理面板</title>

    <!-- PWA相关配置 -->
    <meta name="theme-color" content="{{PRIMARY_COLOR}}">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="{{STATION_NAME}}">
    <link rel="manifest" href="/manifest.json">
    <link rel="icon" href="/favicon.ico" type="image/x-icon">
    <link rel="apple-touch-icon" href="/favicon.ico">
    <style>
        /* 接收后端传来的自定义颜色变量 */
        :root {
            --primary-color: {{PRIMARY_COLOR}};
            --secondary-color: {{SECONDARY_COLOR}};
            --bg-color: {{BG_COLOR}};
        }
        
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            background: var(--bg-color);
            min-height: 100vh;
            padding: 20px;
            color: #333;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 30px;
            padding: 20px;
            background: white;
            border-radius: 15px;
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
        }
        .header-left h1 {
            font-size: 2.2em;
            color: var(--primary-color);
            margin-bottom: 5px;
        }
        .header-left p {
            color: #666;
        }
        .admin-info {
            display: flex;
            align-items: center;
            gap: 15px;
        }
        .admin-badge {
            background: var(--primary-color);
            color: white;
            padding: 8px 15px;
            border-radius: 20px;
            font-size: 0.9em;
        }
        .logout-button {
            background: #e74c3c;
            color: white;
            border: none;
            padding: 8px 20px;
            border-radius: 20px;
            cursor: pointer;
            text-decoration: none;
            font-size: 0.9em;
        }
        .logout-button:hover {
            background: #c0392b;
        }
        .dashboard {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }
        .card {
            background: white;
            border-radius: 15px;
            padding: 25px;
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
        }
        .card h2 {
            color: #495057;
            margin-bottom: 15px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .stat-value {
            font-size: 2.5em;
            font-weight: bold;
            color: var(--secondary-color);
            margin: 10px 0;
        }
        .stat-label {
            color: #6c757d;
            font-size: 0.9em;
        }
        .playlist-section {
            background: white;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
        }
        .playlist-section h2 {
            color: #495057;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        #playlist {
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
            gap: 12px;
            max-height: 400px;
            overflow-y: auto;
            padding-right: 10px;
        }
        .track {
            background: #f8f9fa;
            padding: 15px;
            border-radius: 8px;
            transition: all 0.2s ease;
        }
        .track.current {
            background: rgba(102, 126, 234, 0.1);
            border-left: 4px solid var(--secondary-color);
        }
        .track-header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 10px;
        }
        .track-number {
            background: var(--secondary-color);
            color: white;
            padding: 2px 8px;
            border-radius: 4px;
            font-size: 0.9em;
        }
        .track-title {
            font-weight: 600;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .track-controls {
            display: flex;
            gap: 8px;
            margin-top: 10px;
        }
        .control-button {
            padding: 6px 12px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.9em;
        }
        .play-button {
            background: var(--primary-color);
            color: white;
        }
        .delete-button {
            background: #dc3545;
            color: white;
        }
        .upload-section {
            background: white;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
        }
        .upload-section h2 {
            color: #495057;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        #uploadForm {
            display: flex;
            gap: 15px;
            flex-wrap: wrap;
        }
        input[type="file"] {
            flex-grow: 1;
            padding: 12px;
            border: 2px solid #e9ecef;
            border-radius: 8px;
        }
        button[type="submit"] {
            background: var(--primary-color);
            color: white;
            border: none;
            padding: 12px 25px;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
        }
        #uploadStatus {
            margin-top: 15px;
            padding: 12px;
            border-radius: 8px;
        }
        .success { background: #d4edda; color: #155724; }
        .error { background: #f8d7da; color: #721c24; }
        .info { background: #d1ecf1; color: #0c5460; }
        .bottom-controls {
            display: flex;
            gap: 15px;
            justify-content: center;
            margin-top: 30px;
        }
        .bottom-controls button {
            background: var(--secondary-color);
            color: white;
            border: none;
            padding: 12px 25px;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
        }
        .ncm-section {
            background: white;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }
        .ncm-section h2 {
            color: #495057;
            margin-bottom: 6px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .ncm-section > p {
            color: #6c757d;
            font-size: 0.88em;
            margin-bottom: 18px;
        }
        .ncm-badge {
            display: inline-block;
            padding: 4px 12px;
            border-radius: 12px;
            font-size: 0.82em;
            font-weight: 600;
            margin-bottom: 16px;
        }
        .ncm-badge.ok   { background: #d4edda; color: #155724; }
        .ncm-badge.none { background: #e2e3e5; color: #495057; }
        .ncm-tabs {
            display: flex;
            gap: 10px;
            margin-bottom: 16px;
        }
        .ncm-tab {
            padding: 7px 18px;
            border: 2px solid #e9ecef;
            border-radius: 20px;
            cursor: pointer;
            font-size: 0.88em;
            background: white;
            color: #495057;
        }
        .ncm-tab.active {
            border-color: var(--primary-color);
            color: var(--primary-color);
            font-weight: 600;
        }
        .ncm-panel { display: none; }
        .ncm-panel.active { display: block; }
        .ncm-panel label {
            display: block;
            font-size: 0.88em;
            color: #495057;
            margin-bottom: 5px;
        }
        .ncm-panel input[type="text"],
        .ncm-panel input[type="password"],
        .ncm-panel textarea {
            width: 100%;
            padding: 10px 12px;
            border: 2px solid #e9ecef;
            border-radius: 8px;
            font-size: 0.9em;
            box-sizing: border-box;
            margin-bottom: 12px;
            font-family: inherit;
        }
        .ncm-panel textarea { font-family: monospace; resize: vertical; }
        .ncm-panel small {
            display: block;
            color: #6c757d;
            font-size: 0.8em;
            margin-top: -8px;
            margin-bottom: 12px;
        }
        .ncm-actions {
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
            align-items: center;
        }
        .ncm-actions button {
            padding: 9px 20px;
            border: none;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
            font-size: 0.9em;
        }
        #ncmSaveBtn  { background: var(--primary-color); color: white; }
        #ncmTestBtn  { background: #17a2b8; color: white; }
        #ncmTestBtn:disabled { opacity: 0.5; cursor: not-allowed; }
        #ncmResult {
            margin-top: 12px;
            padding: 10px 14px;
            border-radius: 8px;
            font-size: 0.88em;
            display: none;
        }
        .download-section {
            background: white;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }
        .download-section h2 {
            color: #495057;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        #playlistInput {
            width: 100%;
            padding: 12px;
            border: 2px solid #e9ecef;
            border-radius: 8px;
            font-family: monospace;
            font-size: 0.9em;
            resize: vertical;
            box-sizing: border-box;
        }
        .download-options {
            display: flex;
            gap: 15px;
            flex-wrap: wrap;
            align-items: center;
            margin-top: 12px;
        }
        .download-options label {
            display: flex;
            align-items: center;
            gap: 6px;
            font-size: 0.9em;
            color: #495057;
        }
        .download-options select {
            padding: 8px 12px;
            border: 2px solid #e9ecef;
            border-radius: 6px;
            background: white;
        }
        #startDownloadBtn {
            background: var(--primary-color);
            color: white;
            border: none;
            padding: 10px 22px;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
        }
        #startDownloadBtn:disabled {
            opacity: 0.5;
            cursor: not-allowed;
        }
        #downloadStatus {
            margin-top: 12px;
            padding: 10px 14px;
            border-radius: 8px;
            display: none;
        }
        #downloadLog {
            margin-top: 12px;
            background: #1e1e1e;
            color: #d4d4d4;
            padding: 14px;
            border-radius: 8px;
            font-size: 0.82em;
            max-height: 280px;
            overflow-y: auto;
            white-space: pre-wrap;
            word-break: break-all;
            display: none;
        }

        /* 移动端响应式适配 */
        @media (max-width: 768px) {
            .container {
                padding: 15px;
            }

            header {
                flex-direction: column;
                gap: 20px;
                text-align: center;
            }

            .admin-info {
                justify-content: center;
                flex-wrap: wrap;
            }

            .dashboard {
                grid-template-columns: 1fr;
                gap: 15px;
            }

            .controls {
                flex-direction: column;
                gap: 10px;
            }

            .playlist-section {
                overflow-x: auto;
            }

            .playlist-table {
                min-width: 600px;
            }

            .playlist-table th,
            .playlist-table td {
                padding: 10px 8px;
                font-size: 0.9em;
            }

            .playlist-actions button,
            .upload-controls button,
            .batch-actions button {
                width: 100%;
                margin-bottom: 8px;
            }

            .ncm-settings {
                flex-direction: column;
                gap: 15px;
            }

            .card {
                padding: 20px 15px;
            }

            .stat-value {
                font-size: 2em;
            }
        }

        .settings-section {
            background: white;
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 30px;
            box-shadow: 0 5px 15px rgba(0, 0, 0, 0.1);
        }
        .settings-section h2 {
            color: #495057;
            margin-bottom: 20px;
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .settings-form .form-group {
            margin-bottom: 20px;
        }
        .settings-form label {
            display: block;
            margin-bottom: 8px;
            font-weight: 600;
            color: #495057;
        }
        .settings-form input[type="text"],
        .settings-form input[type="password"] {
            width: 100%;
            padding: 12px 15px;
            border: 2px solid #e9ecef;
            border-radius: 8px;
            font-size: 1em;
            transition: border-color 0.3s;
            box-sizing: border-box;
        }
        .settings-form input[type="text"]:focus,
        .settings-form input[type="password"]:focus {
            border-color: var(--primary-color);
            outline: none;
        }
        .color-group {
            display: flex;
            flex-wrap: wrap;
            gap: 15px;
            align-items: center;
        }
        .color-group input[type="color"] {
            width: 40px;
            height: 40px;
            border: 2px solid #e9ecef;
            border-radius: 6px;
            cursor: pointer;
        }
        .color-group span {
            font-size: 0.85em;
            color: #6c757d;
        }
        .settings-form input[type="checkbox"] {
            margin-right: 10px;
        }
        .form-actions {
            display: flex;
            gap: 15px;
            margin-top: 25px;
        }
        .form-actions button {
            padding: 12px 25px;
            border: none;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
            transition: all 0.3s ease;
        }
        .form-actions .save-btn {
            background: var(--primary-color);
            color: white;
        }
        .form-actions .save-btn:hover {
            background: var(--secondary-color);
            transform: translateY(-2px);
        }
        .form-actions button:first-child {
            background: #6c757d;
            color: white;
        }
        .form-actions button:first-child:hover {
            background: #5a6268;
        }
        .result-message {
            margin-top: 20px;
            padding: 12px;
            border-radius: 6px;
            display: none;
        }
        .result-message.success {
            background: #d4edda;
            color: #155724;
            border: 1px solid #c3e6cb;
            display: block;
        }
        .result-message.error {
            background: #f8d7da;
            color: #721c24;
            border: 1px solid #f5c6cb;
            display: block;
        }
        .result-message.info {
            background: #d1ecf1;
            color: #0c5460;
            border: 1px solid #bee5eb;
            display: block;
        }

        @media (max-width: 480px) {
            .header-left h1 {
                font-size: 1.8em;
            }

            .card h2 {
                font-size: 1.2em;
            }

            .logout-button,
            .admin-badge {
                font-size: 0.9em;
                padding: 6px 12px;
            }

            .upload-section input[type="file"] {
                padding: 10px;
            }

            #downloadLog {
                font-size: 0.8em;
                max-height: 200px;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <div class="header-left">
                <h1>{{STATION_NAME}}</h1>
                <p>管理面板</p>
            </div>
            <div class="admin-info">
                <span class="admin-badge">管理员</span>
                <button onclick="logout()" class="logout-button">退出登录</button>
            </div>
        </header>
        
        <div class="dashboard">
            <div class="card">
                <h2>👥 在线听众</h2>
                <div class="stat-value" id="clientCount">0</div>
                <div class="stat-label">当前连接数</div>
            </div>
            <div class="card">
                <h2>📊 播放统计</h2>
                <div class="stat-value" id="trackCount">0</div>
                <div class="stat-label">总歌曲数</div>
            </div>
            <div class="card">
                <h2>🎵 当前播放</h2>
                <div class="stat-value" id="currentIndex">-</div>
                <div class="stat-label">当前歌曲编号</div>
            </div>
        </div>
        
        <div class="playlist-section">
            <h2>📋 播放列表管理</h2>
            <div id="playlist">加载中...</div>
        </div>
        
        <div class="upload-section">
            <h2>📤 上传新音乐</h2>
            <form id="uploadForm">
                <input type="file" id="fileInput" name="file" accept=".mp3,.wav,.flac,.ogg,.m4a,.aac" required>
                <button type="submit">上传文件</button>
            </form>
            <div id="uploadStatus"></div>
        </div>
        
        <div class="ncm-section">
            <h2>🎵 网易云账号</h2>
            <p>登录后可下载 VIP 歌曲。Cookie 方式更稳定，手机号方式可能触发验证码。</p>
            <div id="ncmBadge" class="ncm-badge none">未配置</div>

            <div class="ncm-tabs">
                <button class="ncm-tab active" onclick="switchNcmTab('cookie', this)">Cookie（推荐）</button>
                <button class="ncm-tab"        onclick="switchNcmTab('phone',  this)">手机号 + 密码</button>
            </div>

            <div id="ncmPanelCookie" class="ncm-panel active">
                <label>Cookie</label>
                <textarea id="ncmCookie" rows="4" placeholder="粘贴网易云 Cookie 字符串..."></textarea>
                <small>获取方式：浏览器打开 <b>music.163.com</b> 并登录 → F12 → Network → 任意请求 → Request Headers → Cookie</small>
            </div>

            <div id="ncmPanelPhone" class="ncm-panel">
                <label>手机号</label>
                <input type="text" id="ncmPhone" placeholder="186xxxxxxxx">
                <label>密码</label>
                <input type="password" id="ncmPassword" placeholder="网易云密码">
            </div>

            <div class="ncm-actions">
                <button id="ncmSaveBtn" onclick="saveNcmSettings()">保存</button>
                <button id="ncmTestBtn" onclick="testNcmLogin()">测试连接</button>
            </div>
            <div id="ncmResult"></div>
        </div>

        <div class="download-section">
            <h2>⬇️ 批量下载歌单</h2>
            <textarea id="playlistInput" rows="8"
                placeholder="每行一首，格式：艺术家 - 歌名&#10;例：&#10;toe - tremolo+delay&#10;Whale Fall - True Places&#10;rinri - 君の世界は透明なんだね"></textarea>
            <div class="download-options">
                <label>音质：
                    <select id="qualitySelect">
                        <option value="exhigh">超高音质 (320k)</option>
                        <option value="lossless">无损</option>
                        <option value="high">高音质 (192k)</option>
                        <option value="standard">标准 (128k)</option>
                    </select>
                </label>
                <label>备用格式：
                    <select id="formatSelect">
                        <option value="mp3">MP3</option>
                        <option value="flac">FLAC</option>
                        <option value="m4a">M4A</option>
                        <option value="opus">Opus</option>
                    </select>
                </label>
                <button id="startDownloadBtn" onclick="startDownload()">开始下载</button>
            </div>
            <div id="downloadStatus"></div>
            <pre id="downloadLog"></pre>
        </div>

        <div class="settings-section">
            <h2>⚙️ 系统设置</h2>
            <div class="settings-form">
                <div class="form-group">
                    <label>站点名称:</label>
                    <input type="text" id="stationName" placeholder="Rakuraku Music Station">
                </div>
                <div class="form-group">
                    <label>副标题:</label>
                    <input type="text" id="subtitle" placeholder="极简流媒体服务器">
                </div>
                <div class="form-group color-group">
                    <label>主题颜色:</label>
                    <input type="color" id="primaryColor" value="#764ba2">
                    <span>主色</span>
                    <input type="color" id="secondaryColor" value="#667eea">
                    <span>次色</span>
                    <input type="color" id="bgColor" value="#f4f4f9">
                    <span>背景色</span>
                </div>
                <div class="form-group">
                    <label>
                        <input type="checkbox" id="allowGuestSkip">
                        允许游客切歌
                    </label>
                </div>
                <div class="form-group">
                    <label>管理员密码:</label>
                    <input type="password" id="adminPassword" placeholder="留空则不修改密码">
                </div>
                <div class="form-actions">
                    <button onclick="loadSettings()">加载设置</button>
                    <button onclick="saveSettings()" class="save-btn">保存设置</button>
                </div>
                <div id="settingsResult" class="result-message"></div>
            </div>
        </div>

        <div class="bottom-controls">
            <button onclick="playNext()">下一首</button>
            <button onclick="playPrev()">上一首</button>
        </div>
    </div>

    <script>
        async function loadPlaylist() {
            try {
                const response = await fetch('/api/playlist');
                const data = await response.json();
                const playlist = data.playlist || [];
                const currentIndex = data.current || 0;
                
                document.getElementById('trackCount').textContent = playlist.length;
                document.getElementById('clientCount').textContent = 0; // 稍后通过stats更新
                document.getElementById('currentIndex').textContent = playlist.length > 0 ? currentIndex + 1 : '-';
                
                let html = '';
                if (playlist.length === 0) {
                    html = '<div style="text-align: center; padding: 40px; color: #6c757d;">播放列表为空，请上传音乐文件</div>';
                } else {
                    const metadata = data.metadata || [];
                    const tracks = metadata.length > 0 ? metadata : playlist.map(f => ({ title: f, artist: '' }));
                    tracks.forEach((track, index) => {
                        const isCurrent = index === currentIndex;
                        const title = track.title || track.filename || playlist[index];
                        const artist = track.artist || '';
                        const displayText = artist ? `${artist} - ${title}` : title;
                        html += `
                            <div class="track ${isCurrent ? 'current' : ''}">
                                <div class="track-header">
                                    <span class="track-number">#${index + 1}</span>
                                    <div class="track-title" title="${displayText}">${displayText}</div>
                                </div>
                                <div class="track-controls">
                                    <button onclick="playTrack(${index})" class="control-button play-button">▶️ 播放</button>
                                    <button onclick="deleteTrack(${index})" class="control-button delete-button">🗑️ 删除</button>
                                </div>
                            </div>
                        `;
                    });
                }
                document.getElementById('playlist').innerHTML = html;
            } catch (error) {
                console.error('加载播放列表失败:', error);
            }
        }
        
        async function loadStats() {
            try {
                const response = await fetch('/api/stats');
                const data = await response.json();
                document.getElementById('clientCount').textContent = data.clients || 0;
            } catch (error) {
                console.error('加载统计失败:', error);
            }
        }
        
        async function playTrack(index) {
            try {
                await fetch('/api/play/' + index, { method: 'POST' });
                setTimeout(loadPlaylist, 500);
            } catch (error) {
                console.error('播放失败:', error);
            }
        }
        
        async function playNext() {
            try {
                await fetch('/api/next', { method: 'POST' });
                setTimeout(loadPlaylist, 500);
            } catch (error) {
                console.error('下一首失败:', error);
            }
        }
        
        async function playPrev() {
            try {
                await fetch('/api/prev', { method: 'POST' });
                setTimeout(loadPlaylist, 500);
            } catch (error) {
                console.error('上一首失败:', error);
            }
        }
        
        async function deleteTrack(index) {
            if (!confirm(`确定要删除曲目 #${index + 1}吗？`)) return;
            
            try {
                const response = await fetch('/api/delete/' + index, { method: 'POST' });
                if (response.ok) {
                    loadPlaylist();
                } else {
                    alert('删除失败');
                }
            } catch (error) {
                console.error('删除失败:', error);
            }
        }
        
        async function logout() {
            try {
                await fetch('/admin/logout', { method: 'POST' });
                window.location.href = '/';
            } catch (error) {
                console.error('退出失败:', error);
            }
        }
        
        function showMessage(message, type = 'info') {
            const statusEl = document.getElementById('uploadStatus');
            statusEl.textContent = message;
            statusEl.className = type;
            if (type !== 'info') setTimeout(() => { statusEl.textContent = ''; statusEl.className = ''; }, 5000);
        }
        
        document.getElementById('uploadForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            const fileInput = document.getElementById('fileInput');
            if (!fileInput.files[0]) return showMessage('请选择文件', 'error');
            if (fileInput.files[0].size > 50 * 1024 * 1024) return showMessage('最大50MB', 'error');
            
            const formData = new FormData();
            formData.append('file', fileInput.files[0]);
            showMessage('上传中...', 'info');
            
            try {
                const response = await fetch('/upload', { method: 'POST', body: formData });
                const text = await response.text();
                if (response.ok) {
                    showMessage('✅ ' + text, 'success');
                    fileInput.value = '';
                    setTimeout(loadPlaylist, 1000);
                } else showMessage('❌ ' + text, 'error');
            } catch (error) { showMessage('❌ 上传失败', 'error'); }
        });
        
        // ── 网易云账号 ──────────────────────────────────────
        let ncmActiveTab = 'cookie';

        function switchNcmTab(tab, btn) {
            ncmActiveTab = tab;
            document.querySelectorAll('.ncm-tab').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            document.getElementById('ncmPanelCookie').classList.toggle('active', tab === 'cookie');
            document.getElementById('ncmPanelPhone').classList.toggle('active',  tab === 'phone');
        }

        async function loadNcmStatus() {
            try {
                const res = await fetch('/admin/settings/ncm');
                if (!res.ok) return;
                const d = await res.json();
                const badge = document.getElementById('ncmBadge');
                if (d.configured) {
                    const label = d.method === 'cookie' ? 'Cookie 已配置' : `手机号 ${d.phone_hint || ''} 已配置`;
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
                const res = await fetch('/admin/settings/ncm', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(payload)
                });
                if (res.ok) {
                    showNcmResult('✅ 保存成功', 'success');
                    loadNcmStatus();
                } else {
                    showNcmResult('❌ ' + await res.text(), 'error');
                }
            } catch (e) { showNcmResult('❌ 请求失败', 'error'); }
        }

        async function testNcmLogin() {
            const btn = document.getElementById('ncmTestBtn');
            btn.disabled = true;
            showNcmResult('测试中...', 'info');
            try {
                const res = await fetch('/admin/settings/ncm/test', { method: 'POST' });
                const d = await res.json();
                showNcmResult(
                    (d.success ? '✅ ' : '❌ ') + (d.output || (d.success ? '登录成功' : '登录失败')),
                    d.success ? 'success' : 'error'
                );
            } catch (e) { showNcmResult('❌ 请求失败', 'error'); }
            btn.disabled = false;
        }

        function showNcmResult(msg, type) {
            const el = document.getElementById('ncmResult');
            el.textContent = msg;
            el.className = type;
            el.style.display = 'block';
        }

        // ── 批量下载 ──────────────────────────────────────
        let downloadPoller = null;

        function showDownloadStatus(msg, type) {
            const el = document.getElementById('downloadStatus');
            el.textContent = msg;
            el.className = type;
            el.style.display = msg ? 'block' : 'none';
        }

        async function startDownload() {
            const playlist = document.getElementById('playlistInput').value.trim();
            if (!playlist) return showDownloadStatus('请输入歌单内容', 'error');

            const quality = document.getElementById('qualitySelect').value;
            const format  = document.getElementById('formatSelect').value;

            document.getElementById('startDownloadBtn').disabled = true;
            document.getElementById('downloadLog').style.display = 'block';
            document.getElementById('downloadLog').textContent = '';
            showDownloadStatus('正在提交任务...', 'info');

            try {
                const res = await fetch('/admin/download', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ playlist, quality, format })
                });
                if (res.ok) {
                    showDownloadStatus('下载中，请稍候...', 'info');
                    pollDownloadStatus();
                } else {
                    const text = await res.text();
                    showDownloadStatus('❌ ' + text, 'error');
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
                    const res = await fetch('/admin/download/status');
                    if (!res.ok) return;
                    const data = await res.json();
                    const logEl = document.getElementById('downloadLog');
                    logEl.textContent = data.log || '';
                    logEl.scrollTop = logEl.scrollHeight;
                    if (!data.running) {
                        clearInterval(downloadPoller);
                        downloadPoller = null;
                        document.getElementById('startDownloadBtn').disabled = false;
                        showDownloadStatus('✅ 下载完成', 'success');
                        setTimeout(loadPlaylist, 1500);
                    }
                } catch (e) {}
            }, 2000);
        }

        // 页面加载时检查是否已有任务在跑
        (async () => {
            try {
                const res = await fetch('/admin/download/status');
                if (!res.ok) return;
                const data = await res.json();
                if (data.running) {
                    document.getElementById('startDownloadBtn').disabled = true;
                    document.getElementById('downloadLog').style.display = 'block';
                    document.getElementById('downloadLog').textContent = data.log || '';
                    showDownloadStatus('下载中，请稍候...', 'info');
                    pollDownloadStatus();
                }
            } catch (e) {}
        })();

        // 加载设置
        async function loadSettings() {
            try {
                const res = await fetch('/admin/settings/get');
                if (!res.ok) {
                    showSettingsMessage('加载设置失败', 'error');
                    return;
                }

                const data = await res.json();
                document.getElementById('stationName').value = data.station_name || 'Rakuraku Music Station';
                document.getElementById('subtitle').value = data.subtitle || '极简流媒体服务器';
                document.getElementById('primaryColor').value = data.primary_color || '#764ba2';
                document.getElementById('secondaryColor').value = data.secondary_color || '#667eea';
                document.getElementById('bgColor').value = data.bg_color || '#f4f4f9';
                document.getElementById('allowGuestSkip').checked = data.allow_guest_skip || false;
                document.getElementById('adminPassword').value = '';
            } catch (error) {
                console.error('加载设置失败:', error);
                showSettingsMessage('加载设置失败: ' + error.message, 'error');
            }
        }

        // 保存设置
        async function saveSettings() {
            try {
                const settings = {
                    station_name: document.getElementById('stationName').value.trim(),
                    subtitle: document.getElementById('subtitle').value.trim(),
                    primary_color: document.getElementById('primaryColor').value,
                    secondary_color: document.getElementById('secondaryColor').value,
                    bg_color: document.getElementById('bgColor').value,
                    allow_guest_skip: document.getElementById('allowGuestSkip').checked
                };

                const password = document.getElementById('adminPassword').value.trim();
                if (password) {
                    settings.admin_password = password;
                }

                const res = await fetch('/admin/settings/save', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(settings)
                });

                if (res.ok) {
                    showSettingsMessage('设置保存成功，正在刷新页面...', 'success');
                    setTimeout(() => window.location.reload(), 1000);
                } else {
                    const text = await res.text();
                    showSettingsMessage('保存失败: ' + text, 'error');
                }
            } catch (error) {
                console.error('保存设置失败:', error);
                showSettingsMessage('保存失败: ' + error.message, 'error');
            }
        }

        // 显示设置消息
        function showSettingsMessage(message, type) {
            const el = document.getElementById('settingsResult');
            el.textContent = message;
            el.className = 'result-message ' + type;
            el.style.display = 'block';

            if (type === 'success') {
                setTimeout(() => el.style.display = 'none', 5000);
            }
        }

        // 页面加载完成后自动加载设置
        loadSettings();

        loadPlaylist();
        loadStats();
        loadNcmStatus();
        setInterval(loadPlaylist, 3000);
        setInterval(loadStats, 2000);

        // PWA Service Worker 注册
        if ('serviceWorker' in navigator) {
            window.addEventListener('load', async () => {
                try {
                    const registration = await navigator.serviceWorker.register('/sw.js');
                    console.log('Service Worker 注册成功');
                } catch (error) {
                    console.error('Service Worker 注册失败:', error);
                }
            });
        }
    </script>
</body>
</html>

)RKTML"},
  {"login.html", R"RKTML(
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <title>{{STATION_NAME}} - 管理员登录</title>

    <!-- PWA相关配置 -->
    <meta name="theme-color" content="{{PRIMARY_COLOR}}">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="{{STATION_NAME}}">
    <link rel="manifest" href="/manifest.json">
    <link rel="icon" href="/favicon.ico" type="image/x-icon">
    <link rel="apple-touch-icon" href="/favicon.ico">
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { 
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; 
            background: linear-gradient(135deg, {{PRIMARY_COLOR}} 0%, {{SECONDARY_COLOR}} 100%);
            min-height: 100vh;
            display: flex;
            justify-content: center;
            align-items: center;
            padding: 20px;
        }
        .login-container {
            width: 100%;
            max-width: 400px;
            background: rgba(255, 255, 255, 0.95);
            border-radius: 20px;
            padding: 40px 30px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
        }
        .logo {
            text-align: center;
            margin-bottom: 30px;
        }
        .logo h1 {
            font-size: 2.2em;
            color: {{PRIMARY_COLOR}};
            margin-bottom: 10px;
        }
        .logo p {
            color: #666;
            font-size: 1em;
        }
        .form-group {
            margin-bottom: 25px;
        }
        label {
            display: block;
            margin-bottom: 8px;
            font-weight: 600;
            color: #495057;
        }
        input[type="password"] {
            width: 100%;
            padding: 12px 15px;
            font-size: 1em;
            border: 2px solid #e9ecef;
            border-radius: 8px;
            outline: none;
            transition: border-color 0.3s;
        }
        input[type="password"]:focus {
            border-color: {{PRIMARY_COLOR}};
        }
        .login-button {
            width: 100%;
            background: {{PRIMARY_COLOR}};
            color: white;
            border: none;
            padding: 14px;
            border-radius: 8px;
            font-size: 1.1em;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
        }
        .login-button:hover {
            background: {{SECONDARY_COLOR}};
            transform: translateY(-2px);
        }
        .error-message {
            margin-top: 15px;
            padding: 10px;
            background: #f8d7da;
            color: #721c24;
            border-radius: 5px;
            text-align: center;
            display: none;
        }
        .back-link {
            text-align: center;
            margin-top: 20px;
        }
        .back-link a {
            color: #6c757d;
            text-decoration: none;
        }
        .back-link a:hover {
            text-decoration: underline;
            color: {{SECONDARY_COLOR}};
        }

        /* 移动端响应式适配 */
        @media (max-width: 768px) {
            body {
                padding: 15px;
            }

            .login-container {
                padding: 30px 20px;
                margin: 0 10px;
            }

            .logo h1 {
                font-size: 1.8em;
            }

            .logo p {
                font-size: 0.9em;
            }

            input[type="password"] {
                padding: 14px 12px;
                font-size: 1.1em;
            }

            .login-button {
                padding: 16px;
                font-size: 1.1em;
            }
        }

        @media (max-width: 480px) {
            body {
                padding: 10px;
            }

            .login-container {
                padding: 25px 15px;
                margin: 0;
                border-radius: 15px;
            }

            .logo h1 {
                font-size: 1.6em;
            }

            input[type="password"],
            .login-button {
                font-size: 1em;
            }
        }
    </style>
</head>
<body>
    <div class="login-container">
        <div class="logo">
            <h1>{{STATION_NAME}}</h1>
            <p>{{SUBTITLE}}</p>
            <p style="margin-top: 10px; color: #495057;">管理员登录</p>
        </div>
        
        <form id="loginForm">
            <div class="form-group">
                <label for="password">密码</label>
                <input type="password" id="password" placeholder="请输入管理员密码" required>
            </div>
            
            <button type="submit" class="login-button">登录</button>
            
            <div id="errorMessage" class="error-message"></div>
        </form>
        
        <div class="back-link">
            <a href="/">← 返回主页</a>
        </div>
    </div>

    <script>
        document.getElementById('loginForm').addEventListener('submit', async (e) => {
            e.preventDefault();
            
            const password = document.getElementById('password').value.trim();
            const errorElement = document.getElementById('errorMessage');
            
            if (!password) {
                errorElement.textContent = "请输入密码";
                errorElement.style.display = "block";
                return;
            }
            
            try {
                const response = await fetch('/admin/login', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json'
                    },
                    body: JSON.stringify({ password: password })
                });
                
                if (response.ok) {
                    // 登录成功，跳转到管理面板
                    window.location.href = '/';
                } else {
                    const errorText = await response.text();
                    errorElement.textContent = errorText;
                    errorElement.style.display = "block";
                }
            } catch (error) {
                errorElement.textContent = "登录请求失败";
                errorElement.style.display = "block";
            }
        });
    </script>
</body>
</html>

)RKTML"},
  {"manifest.json", R"RKTML(
{
  "name": "{{STATION_NAME}}",
  "short_name": "RakurakuRadio",
  "description": "轻松音乐电台 - 低延迟音频流媒体",
  "start_url": "/",
  "scope": "/",
  "display": "standalone",
  "background_color": "{{BG_COLOR}}",
  "theme_color": "{{PRIMARY_COLOR}}",
  "icons": [
    {
      "src": "/icon.svg",
      "sizes": "any",
      "type": "image/svg+xml",
      "purpose": "any"
    }
  ]
}

)RKTML"},
  {"sw.js", R"RKTML(
const CACHE_NAME = 'rakuraku-v1';

// 这些路径走纯网络，不缓存
const BYPASS = ['/stream', '/api/', '/admin/download', '/upload'];

self.addEventListener('install', event => {
    event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', event => {
    event.waitUntil(
        caches.keys()
            .then(keys => Promise.all(
                keys.filter(k => k !== CACHE_NAME).map(k => caches.delete(k))
            ))
            .then(() => self.clients.claim())
    );
});

self.addEventListener('fetch', event => {
    if (event.request.method !== 'GET') return;

    const { pathname } = new URL(event.request.url);
    if (BYPASS.some(p => pathname.startsWith(p))) {
        event.respondWith(fetch(event.request));
        return;
    }

    // 网络优先：成功则更新缓存，失败则回退到缓存
    event.respondWith(
        fetch(event.request)
            .then(res => {
                if (res.ok) {
                    const clone = res.clone();
                    caches.open(CACHE_NAME).then(c => c.put(event.request, clone));
                }
                return res;
            })
            .catch(() => caches.match(event.request))
    );
});

)RKTML"},
};
}
