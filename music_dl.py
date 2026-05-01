#!/usr/bin/env python3
"""
music_dl.py — 从 txt/csv 歌单批量搜索并下载音乐

音源降级链：网易云音乐 → YouTube（可访问时）→ Bilibili（YouTube 不可达时兜底）
交互模式：展示搜索结果列表，用户手动选择；非交互模式自动选第一项
"""

import sys
import re
import csv
import json
import time
import hashlib
import argparse
import subprocess
import urllib.request
import urllib.parse
import urllib.error
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, Callable, Any

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(line_buffering=True)

try:
    from pyncm.apis import cloudsearch, track as ncm_track
    HAS_PYNCM = True
except ImportError:
    HAS_PYNCM = False

QUALITY_BITRATE = {
    "standard": 128000,
    "high":     192000,
    "exhigh":   320000,
    "lossless": 999000,
}

SAFE_CHARS = re.compile(r'[\\/:*?"<>|]')
HTML_TAG = re.compile(r"<[^>]+>")

YOUTUBE_CHECK_TIMEOUT = 5
YTDLP_TIMEOUT = 300
SEARCH_TIMEOUT = 60


# ── 数据结构 ──────────────────────────────────────────────

@dataclass
class Track:
    artist: Optional[str]
    title: str

    def query(self) -> str:
        return f"{self.artist} {self.title}" if self.artist else self.title

    def label(self) -> str:
        return f"{self.artist} - {self.title}" if self.artist else self.title

    def safe_filename(self, ext: str) -> str:
        return SAFE_CHARS.sub("_", self.label()) + f".{ext}"


# ── 文件解析 ──────────────────────────────────────────────

def parse_txt(filepath: Path) -> list[Track]:
    dash_re = re.compile(r"^(.+?)\s+-\s+(.+)$")
    tracks: list[Track] = []
    with open(filepath, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            m = dash_re.match(line)
            if m:
                tracks.append(Track(artist=m.group(1).strip(), title=m.group(2).strip()))
            else:
                tracks.append(Track(artist=None, title=line))
    return tracks


def parse_csv(filepath: Path) -> list[Track]:
    ARTIST_KEYS = {"artist", "artists", "singer", "歌手", "艺术家", "艺人"}
    TITLE_KEYS  = {"title", "name", "song", "track", "歌名", "曲名", "标题", "song name"}
    with open(filepath, encoding="utf-8-sig") as f:
        reader = csv.DictReader(f)
        fields = {c.lower().strip(): c for c in (reader.fieldnames or [])}
        artist_col = next((fields[k] for k in ARTIST_KEYS if k in fields), None)
        title_col  = next((fields[k] for k in TITLE_KEYS  if k in fields), None)
        if not title_col and reader.fieldnames:
            title_col = reader.fieldnames[0]
        tracks: list[Track] = []
        for row in reader:
            title  = row.get(title_col, "").strip() if title_col else ""
            artist = row.get(artist_col, "").strip() if artist_col else None
            if title:
                tracks.append(Track(artist=artist or None, title=title))
    return tracks


def parse_file(filepath: Path) -> list[Track]:
    if filepath.suffix.lower() == ".csv":
        return parse_csv(filepath)
    return parse_txt(filepath)


# ── 工具函数 ──────────────────────────────────────────────

def is_tty() -> bool:
    return sys.stdin.isatty() and sys.stdout.isatty()


def strip_html(text: str) -> str:
    return HTML_TAG.sub("", text)


def check_youtube_reachable(timeout: int = YOUTUBE_CHECK_TIMEOUT) -> bool:
    try:
        req = urllib.request.Request("https://www.youtube.com/",
                                     headers={"User-Agent": "Mozilla/5.0"})
        urllib.request.urlopen(req, timeout=timeout)
        return True
    except Exception:
        return False


def run_ytdlp(args: list[str], timeout: int = YTDLP_TIMEOUT) -> tuple[int, str, str]:
    try:
        r = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "yt-dlp 超时"
    except FileNotFoundError:
        return -1, "", "yt-dlp 未安装"
    except Exception as e:
        return -1, "", str(e)


# ── 交互选择 ──────────────────────────────────────────────

def show_and_select(items: list[Any], display_fn: Callable[[Any], str],
                    label: str = "此项", max_show: int = 15) -> Optional[Any]:
    """展示搜索结果列表供用户选择。非 TTY 时自动选第一项。返回选中项或 None（跳过）。"""
    if not items:
        return None

    count = min(len(items), max_show)

    if not is_tty():
        for i, item in enumerate(items[:count], 1):
            print(f"    [{i}] {display_fn(item)}")
        if len(items) > max_show:
            print(f"    ... 还有 {len(items) - max_show} 项未显示")
        print(f"    → 非交互模式，自动选择第 1 项")
        return items[0]

    for i, item in enumerate(items[:count], 1):
        print(f"  {i:>2}. {display_fn(item)}")
    if len(items) > max_show:
        print(f"      ... 还有 {len(items) - max_show} 项未显示")
    print(f"   0. 跳过{label}")

    while True:
        try:
            choice = input("  请选择: ").strip()
            if not choice:
                continue
            idx = int(choice)
            if idx == 0:
                return None
            if 1 <= idx <= len(items):
                return items[idx - 1]
            print(f"  请输入 1-{len(items)} 或 0")
        except (ValueError, EOFError):
            print("  输入无效，请输入数字")
        except KeyboardInterrupt:
            print()
            raise


# ── 网易云账号 ────────────────────────────────────────────

def load_ncm_credentials(settings_path: str) -> tuple[str, str, str]:
    if not settings_path:
        return "", "", ""
    try:
        with open(settings_path, encoding="utf-8") as f:
            s = json.load(f)
        return s.get("ncm_phone", ""), s.get("ncm_password", ""), s.get("ncm_cookie", "")
    except Exception:
        return "", "", ""


def parse_browser_cookies(cookie_str: str) -> dict:
    cookies: list[dict] = []
    csrf_token = ""
    for c in cookie_str.split(';'):
        c = c.strip()
        if not c or '=' not in c:
            continue
        k, v = c.split('=', 1)
        cookies.append({"name": k, "value": v, "domain": "music.163.com", "path": "/"})
        if k == "__csrf":
            csrf_token = v

    session_dict = {
        "eapi_config": {
            "os": "Android", "appver": "8.9.70", "osver": "11",
            "channel": "netease", "deviceId": ""
        },
        "login_info": {"success": True, "tick": time.time(), "content": None},
        "csrf_token": csrf_token, "cookies": cookies
    }
    try:
        import pyncm
        session = pyncm.GetCurrentSession()
        real_dump = session.dump()
        session_dict["eapi_config"] = real_dump.get("eapi_config", session_dict["eapi_config"])
    except ImportError:
        pass
    return session_dict


def ncm_authenticate(phone: str, password: str, cookie: str) -> bool:
    if not HAS_PYNCM:
        return False
    try:
        import pyncm
        from pyncm.apis import login as ncm_login
        if cookie and cookie.strip():
            cookie = cookie.strip()
            session = pyncm.GetCurrentSession()
            parsed = False
            if '=' in cookie and (';' in cookie or '__csrf=' in cookie or 'MUSIC_U=' in cookie):
                try:
                    session.load(parse_browser_cookies(cookie))
                    parsed = True
                except Exception as e:
                    print(f"[网易云] 浏览器Cookie解析失败：{e}")
            if not parsed:
                try:
                    session.load(json.loads(cookie) if isinstance(cookie, str) else cookie)
                    parsed = True
                except (json.JSONDecodeError, Exception) as e:
                    print(f"[网易云] Cookie解析失败：{e}")
            if not parsed:
                print("[网易云] Cookie 格式错误")
                return False
            status = ncm_login.GetCurrentLoginStatus()
            if status.get("account"):
                nick = status.get("profile", {}).get("nickname", "未知")
                print(f"[网易云] Cookie 登录成功：{nick}")
                return True
            print("[网易云] Cookie 无效或已过期")
            return False

        if phone and password:
            r = ncm_login.LoginViaCellphone(phone=phone, password=password)
            if r.get("code") == 200:
                nick = r.get("profile", {}).get("nickname", "未知")
                print(f"[网易云] 手机号登录成功：{nick}")
                return True
            msg = r.get("message") or r.get("msg") or str(r.get("code"))
            print(f"[网易云] 登录失败：{msg}")
            return False
    except Exception as e:
        print(f"[网易云] 登录异常：{e}")
    return False


# ── 网易云搜索 + 下载 ─────────────────────────────────────

def ncm_search(query: str, limit: int = 15) -> list[dict]:
    if not HAS_PYNCM:
        return []
    try:
        r = cloudsearch.GetSearchResult(keyword=query, stype=1, limit=limit)
        return r.get("result", {}).get("songs", [])
    except Exception as e:
        print(f"  [网易云搜索错误] {e}")
        return []


def ncm_get_audio(song_id: int, bitrate: int) -> tuple[Optional[str], str, bool]:
    try:
        r = ncm_track.GetTrackAudio([song_id], bitrate=bitrate)
        data = r.get("data", [])
        if data and data[0].get("url"):
            d = data[0]
            return d["url"], d.get("type", "mp3"), bool(d.get("freeTrialInfo"))
    except Exception as e:
        print(f"  [获取链接错误] {e}")
    return None, "mp3", False


def _ncm_display(song: dict) -> str:
    artists = " / ".join(a["name"] for a in song.get("ar", []))
    name = song.get("name", "?")
    sid = song.get("id", "?")
    return f"{artists} - {name} (id:{sid})"


def ncm_download(t: Track, output_dir: Path, bitrate: int,
                 authenticated: bool = False) -> bool:
    songs = ncm_search(t.query())
    if not songs:
        print("  → 网易云未找到结果")
        return False

    chosen = show_and_select(songs, _ncm_display, label="网易云结果")
    if chosen is None:
        print("  → 用户跳过")
        return True

    song_id   = chosen["id"]
    ar_names  = " / ".join(a["name"] for a in chosen.get("ar", []))
    song_name = chosen.get("name", "")
    print(f"  → 已选择: {ar_names} - {song_name} (id:{song_id})")

    url, ext, is_trial = ncm_get_audio(song_id, bitrate)
    if not url:
        hint = "该歌曲无版权或账号无权限" if authenticated else "无可用链接（可能需要登录VIP账号）"
        print(f"  → {hint}")
        return False
    if is_trial:
        hint = "账号无此歌曲VIP权限" if authenticated else "仅试听片段，需要登录网易云VIP账号"
        print(f"  → {hint}")
        return False

    dest = output_dir / SAFE_CHARS.sub("_", f"{ar_names} - {song_name}.{ext}")
    if dest.exists():
        print(f"  → 已存在，跳过")
        return True

    try:
        urllib.request.urlretrieve(url, dest)
        print(f"  → 下载完成: {dest.name}")
        return True
    except Exception as e:
        print(f"  → 下载失败: {e}")
        if dest.exists():
            dest.unlink()
        return False


# ── Bilibili WBI 签名 ─────────────────────────────────────

class BilibiliWBI:
    def __init__(self):
        self._img_key: str = ""
        self._sub_key: str = ""
        self._fetched: bool = False

    def _fetch_keys(self):
        if self._fetched:
            return
        try:
            req = urllib.request.Request("https://api.bilibili.com/x/web-interface/nav",
                                         headers={"User-Agent": "Mozilla/5.0"})
            with urllib.request.urlopen(req, timeout=10) as r:
                nav = json.loads(r.read())
            wbi_img = nav.get("data", {}).get("wbi_img", {})
            img_url = wbi_img.get("img_url", "")
            sub_url = wbi_img.get("sub_url", "")
            m = re.search(r"/wbi/([^/.]+)", img_url)
            if m:
                self._img_key = m.group(1)
            m = re.search(r"/wbi/([^/.]+)", sub_url)
            if m:
                self._sub_key = m.group(1)
            if self._img_key and self._sub_key:
                self._fetched = True
        except Exception as e:
            print(f"  [Bilibili WBI 密钥获取失败: {e}]")

    def sign(self, params: dict) -> dict:
        self._fetch_keys()
        if not self._img_key:
            return params
        params["wts"] = int(time.time())
        mix = self._img_key + self._sub_key
        qs = "&".join(f"{k}={params[k]}" for k in sorted(params.keys()))
        params["w_rid"] = hashlib.md5((qs + mix).encode()).hexdigest()
        return params


_wbi = BilibiliWBI()


# ── yt-dlp 通用搜索 ───────────────────────────────────────

def _search_ytdlp(query: str, extractor_prefix: str, limit: int = 10) -> list[dict]:
    cmd = [
        "yt-dlp", "--flat-playlist", "--dump-json",
        "--no-warnings", "--socket-timeout", "30",
        "--no-check-certificates",
        f"{extractor_prefix}{limit}:{query}"
    ]
    rc, stdout, stderr = run_ytdlp(cmd, timeout=SEARCH_TIMEOUT)
    if rc != 0 or not stdout.strip():
        return []
    results: list[dict] = []
    for line in stdout.strip().split('\n'):
        if not line.strip():
            continue
        try:
            results.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    return results


def _ytdlp_display(item: dict) -> str:
    uploader = item.get("uploader") or item.get("channel") or item.get("uploader_id", "?")
    title = item.get("title") or item.get("fulltitle", "?")
    dur = item.get("duration")
    d = f" [{dur // 60}:{dur % 60:02d}]" if dur else ""
    return f"{uploader} - {title}{d}"


def _ytdlp_download_url(url: str, label: str, output_dir: Path, fmt: str) -> bool:
    safe = SAFE_CHARS.sub("_", label)
    template = str(output_dir / f"{safe}.%(ext)s")
    cmd = [
        "yt-dlp", "--extract-audio", "--audio-format", fmt, "--audio-quality", "0",
        "--output", template, "--no-playlist", "--quiet", "--no-warnings",
        "--socket-timeout", "30", "--no-check-certificates", url
    ]
    rc, _, stderr = run_ytdlp(cmd)
    if rc != 0:
        if stderr:
            print(f"  → yt-dlp 错误: {stderr[:200]}")
        return False
    import os
    base = str(output_dir / safe)
    for test_ext in [fmt, "m4a", "opus", "webm", "mp3"]:
        if Path(f"{base}.{test_ext}").exists():
            return True
        if Path(f"{base}.{fmt}.{test_ext}").exists():
            return True
    return True


# ── YouTube 搜索 + 下载 ───────────────────────────────────

def ytdlp_download(t: Track, output_dir: Path, fmt: str) -> bool:
    print("  → yt-dlp 搜索 YouTube...")
    results = _search_ytdlp(t.query(), "ytsearch", limit=10)
    if not results:
        print("  → YouTube 未找到结果")
        return False

    chosen = show_and_select(results, _ytdlp_display, label="YouTube 结果")
    if chosen is None:
        print("  → 用户跳过")
        return True

    uploader = chosen.get("uploader") or chosen.get("channel", "")
    title = chosen.get("title") or chosen.get("fulltitle", "?")
    url = chosen.get("webpage_url") or chosen.get("url") or \
          f"https://www.youtube.com/watch?v={chosen.get('id', '')}"
    label = f"{uploader} - {title}" if uploader else title
    print(f"  → 已选择: {label}")
    if _ytdlp_download_url(url, label, output_dir, fmt):
        print(f"  → 下载完成")
        return True
    return False


# ── Bilibili 搜索 + 下载 ──────────────────────────────────

def bilibili_search_api(query: str, limit: int = 10) -> list[dict]:
    params = _wbi.sign({"search_type": "video", "keyword": query})
    qs = urllib.parse.urlencode(params)
    url = f"https://api.bilibili.com/x/web-interface/wbi/search/type?{qs}"
    req = urllib.request.Request(url, headers={
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        "Referer": "https://www.bilibili.com/"
    })
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            data = json.loads(r.read())
        if data.get("code") != 0:
            return []
        results: list[dict] = []
        for v in data.get("data", {}).get("result", [])[:limit]:
            bvid = v.get("bvid", "")
            if not bvid:
                continue
            dur_raw = v.get("duration", "")
            results.append({
                "bvid": bvid,
                "title": strip_html(v.get("title", "")),
                "uploader": v.get("author", ""),
                "duration": dur_raw,
                "url": f"https://www.bilibili.com/video/{bvid}",
            })
        return results
    except Exception as e:
        print(f"  [Bilibili 搜索错误: {e}]")
        return []


def _bili_display(item: dict) -> str:
    uploader = item.get("uploader", "?")
    title = item.get("title", "?")
    dur = item.get("duration", "")
    d = f" [{dur}]" if dur else ""
    return f"{uploader} - {title}{d}"


def bilibili_download(t: Track, output_dir: Path, fmt: str) -> bool:
    print("  → 搜索 Bilibili...")
    results = bilibili_search_api(t.query(), limit=10)
    if not results:
        print("  → Bilibili 未找到结果")
        return False

    chosen = show_and_select(results, _bili_display, label="Bilibili 结果")
    if chosen is None:
        print("  → 用户跳过")
        return True

    uploader = chosen.get("uploader", "")
    title = chosen.get("title", "?")
    url = chosen["url"]
    label = f"{uploader} - {title}" if uploader else title
    print(f"  → 已选择: {label} ({url})")

    if _ytdlp_download_url(url, label, output_dir, fmt):
        print(f"  → 下载完成")
        return True
    return False


# ── 主流程 ────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="从 txt/csv 歌单批量下载音乐\n"
                    "音源降级链：网易云 → YouTube → Bilibili\n"
                    "交互模式展示搜索结果，用户自行选择"
    )
    parser.add_argument("file", nargs="?", help="输入文件 (txt 或 csv)")
    parser.add_argument("-o", "--output",   default="downloads", help="输出目录 (默认: downloads)")
    parser.add_argument("-f", "--format",   default="mp3",
                        choices=["mp3", "flac", "m4a", "opus"],
                        help="yt-dlp 下载音频格式 (默认: mp3)")
    parser.add_argument("-q", "--quality",  default="exhigh",
                        choices=list(QUALITY_BITRATE),
                        help="网易云音质: standard=128k / high=192k / exhigh=320k / lossless")
    parser.add_argument("--settings",       help="settings.json 路径，用于读取网易云凭据")
    parser.add_argument("--verify-login",   action="store_true", help="仅测试网易云登录")
    parser.add_argument("--no-ncm",         action="store_true", help="不使用网易云")
    parser.add_argument("--no-ytdlp",       action="store_true", help="不使用 yt-dlp（含Bilibili）")
    parser.add_argument("--no-bilibili",    action="store_true", help="不使用 Bilibili 兜底")
    parser.add_argument("--delay",          type=float, default=1.0, help="每首间隔秒数 (默认: 1.0)")
    parser.add_argument("--dry-run",        action="store_true", help="只解析，不下载")
    parser.add_argument("--non-interactive", action="store_true",
                        help="非交互模式，自动选第一个结果")
    args = parser.parse_args()

    if not HAS_PYNCM:
        print("警告: pyncm 未安装 → pip install pyncm\n")

    authenticated = False
    if HAS_PYNCM and not args.no_ncm:
        phone, password, cookie = load_ncm_credentials(args.settings)
        if phone or cookie:
            authenticated = ncm_authenticate(phone, password, cookie)
        else:
            print("[网易云] 未配置账号，以游客模式下载（VIP歌曲不可用）")

    if args.verify_login:
        sys.exit(0 if authenticated else 1)

    if not args.file:
        parser.print_help()
        sys.exit(0)

    filepath = Path(args.file)
    if not filepath.exists():
        print(f"错误: 找不到文件 {filepath}")
        sys.exit(1)

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    tracks = parse_file(filepath)
    bitrate = QUALITY_BITRATE[args.quality]

    youtube_reachable = False
    if not args.no_ytdlp:
        print("[连通检测] 检查 YouTube...", end=" ", flush=True)
        youtube_reachable = check_youtube_reachable()
        print("可访问" if youtube_reachable else "不可访问")
        if not youtube_reachable and not args.no_bilibili:
            print("[连通检测] Bilibili 将作为兜底音源")

    sources: list[str] = []
    if not args.no_ncm:
        sources.append(f"网易云({'已登录' if authenticated else '游客'})")
    if not args.no_ytdlp and youtube_reachable:
        sources.append("YouTube")
    if not args.no_ytdlp and not youtube_reachable and not args.no_bilibili:
        sources.append("Bilibili(兜底)")
    mode = "非交互" if (args.non_interactive or not is_tty()) else "交互"
    print(f"解析到 {len(tracks)} 首  |  音源: {' → '.join(sources)}  |  输出: {output_dir}  |  模式: {mode}\n", flush=True)

    if args.dry_run:
        for i, t in enumerate(tracks, 1):
            print(f"  {i:>3}. {t.label()}")
        return

    if args.non_interactive:
        import os
        global _saved_stdin
        _saved_stdin = sys.stdin
        sys.stdin = open(os.devnull, 'r')

    failed: list[str] = []

    for i, t in enumerate(tracks, 1):
        print(f"[{i}/{len(tracks)}] {t.label()}", flush=True)
        ok = False

        if not args.no_ncm and HAS_PYNCM:
            ok = ncm_download(t, output_dir, bitrate, authenticated=authenticated)

        if not ok and not args.no_ytdlp and youtube_reachable:
            ok = ytdlp_download(t, output_dir, args.format)

        if not ok and not args.no_ytdlp and not youtube_reachable and not args.no_bilibili:
            ok = bilibili_download(t, output_dir, args.format)

        print("  ✓ 完成" if ok else "  ✗ 失败", flush=True)
        if not ok:
            failed.append(t.label())

        if i < len(tracks):
            time.sleep(args.delay)

    total = len(tracks)
    print(f"\n完成 {total - len(failed)}/{total} 首", flush=True)

    if failed:
        failed_path = output_dir / "failed.txt"
        failed_path.write_text("\n".join(failed), encoding="utf-8")
        print(f"失败 {len(failed)} 首，已写入 {failed_path}：")
        for f in failed:
            print(f"  - {f}")


if __name__ == "__main__":
    main()
