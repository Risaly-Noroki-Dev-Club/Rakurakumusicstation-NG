#!/usr/bin/env python3
"""
music_dl.py — 从 txt/csv 歌单批量搜索并下载音乐
主要音源：网易云音乐（pyncm）
备用音源：YouTube（yt-dlp）
"""

import sys
import re
import csv
import json
import time
import argparse
import subprocess
import urllib.request
from pathlib import Path
from dataclasses import dataclass
from typing import Optional

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


@dataclass
class Track:
    artist: Optional[str]
    title: str

    def query(self) -> str:
        return f"{self.artist} {self.title}" if self.artist else self.title

    def label(self) -> str:
        return f"{self.artist} - {self.title}" if self.artist else self.title

    def safe_filename(self, ext: str) -> str:
        name = SAFE_CHARS.sub("_", self.label())
        return f"{name}.{ext}"


# ── 文件解析 ──────────────────────────────────────────────

def parse_txt(filepath: Path) -> list[Track]:
    dash_re = re.compile(r"^(.+?)\s+-\s+(.+)$")
    tracks = []
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

        tracks = []
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


# ── 网易云账号 ────────────────────────────────────────────

def load_ncm_credentials(settings_path: str) -> tuple[str, str, str]:
    """从 settings.json 读取网易云凭据，返回 (phone, password, cookie)。"""
    if not settings_path:
        return "", "", ""
    try:
        with open(settings_path, encoding="utf-8") as f:
            s = json.load(f)
        return s.get("ncm_phone", ""), s.get("ncm_password", ""), s.get("ncm_cookie", "")
    except Exception:
        return "", "", ""


def parse_browser_cookies(cookie_str: str) -> dict:
    """解析浏览器Cookie字符串为pyncm兼容的session dict格式"""
    import time

    # 解析cookie字符串到列表
    cookies = []
    csrf_token = ""
    for cookie in cookie_str.split(';'):
        cookie = cookie.strip()
        if not cookie or '=' not in cookie:
            continue

        key, value = cookie.split('=', 1)
        cookie_dict = {
            "name": key,
            "value": value,
            "domain": "music.163.com",
            "path": "/"
            # 注意: pyncm dump 不包含 expires 字段
        }
        cookies.append(cookie_dict)

        if key == "__csrf":
            csrf_token = value

    # 构建session结构，模拟 pyncm dump 格式
    session_dict = {
        "eapi_config": {
            "os": "Android",
            "appver": "8.9.70",
            "osver": "11",
            "channel": "netease",
            "deviceId": ""
        },
        "login_info": {
            "success": True,
            "tick": time.time(),
            "content": None
        },
        "csrf_token": csrf_token,
        "cookies": cookies
    }

    # 如果 pyncm 可用，用真实 dump 的结构更新
    try:
        import pyncm
        session = pyncm.GetCurrentSession()
        real_dump = session.dump()
        # 保留真实 dump 的 eapi_config 结构，只更新 cookies 和登录状态
        session_dict["eapi_config"] = real_dump.get("eapi_config", session_dict["eapi_config"])
    except ImportError:
        # pyncm 不可用，使用默认结构
        pass

    return session_dict


def ncm_authenticate(phone: str, password: str, cookie: str) -> bool:
    """尝试登录网易云，成功返回 True。Cookie 优先，次选手机号+密码。"""
    if not HAS_PYNCM:
        return False
    try:
        import pyncm
        from pyncm.apis import login as ncm_login

        if cookie and cookie.strip():
            cookie = cookie.strip()
            session = pyncm.GetCurrentSession()

            # Try multiple cookie formats
            parsed_successfully = False

            # Format 1: Try as browser cookie string (semicolon-separated)
            if '=' in cookie and (';' in cookie or '__csrf=' in cookie or 'MUSIC_U=' in cookie):
                # Looks like a browser cookie string
                try:
                    cookie_dict = parse_browser_cookies(cookie)
                    session.load(cookie_dict)
                    parsed_successfully = True
                except Exception as e:
                    print(f"[网易云] 浏览器Cookie解析失败：{e}")

            # Format 2: Try as JSON string (legacy format)
            if not parsed_successfully:
                try:
                    import json
                    cookie_dict = json.loads(cookie) if isinstance(cookie, str) else cookie
                    session.load(cookie_dict)
                    parsed_successfully = True
                except json.JSONDecodeError:
                    # Not JSON, will fail below
                    pass
                except Exception as e:
                    print(f"[网易云] JSON Cookie解析失败：{e}")

            if not parsed_successfully:
                print("[网易云] Cookie 格式错误：既不是浏览器Cookie格式也不是JSON格式")
                return False

            # Check login status
            status = ncm_login.GetCurrentLoginStatus()
            if status.get("account"):
                nickname = status.get("profile", {}).get("nickname", "未知")
                print(f"[网易云] Cookie 登录成功：{nickname}")
                return True
            else:
                print("[网易云] Cookie 无效或已过期")
                return False

        if phone and password:
            result = ncm_login.LoginViaCellphone(phone=phone, password=password)
            if result.get("code") == 200:
                nickname = result.get("profile", {}).get("nickname", "未知")
                print(f"[网易云] 手机号登录成功：{nickname}")
                return True
            msg = result.get("message") or result.get("msg") or str(result.get("code"))
            print(f"[网易云] 登录失败：{msg}")
            return False

    except Exception as e:
        print(f"[网易云] 登录异常：{e}")
    return False


# ── 搜索相似度评分 ────────────────────────────────────────

def _tokenize(s: str) -> set:
    s = s.lower()
    cjk   = {c for c in s if "一" <= c <= "鿿" or "぀" <= c <= "ヿ"}
    words = set(re.findall(r"[a-z0-9]+", s))
    return cjk | words


def _overlap(a: str, b: str) -> float:
    ta, tb = _tokenize(a), _tokenize(b)
    if not ta:
        return 1.0
    return len(ta & tb) / len(ta)


def score_result(t: Track, r: dict) -> float:
    r_title   = r.get("name", "")
    r_artists = " ".join(a["name"] for a in r.get("ar", []))
    title_sc  = _overlap(t.title, r_title)
    artist_sc = _overlap(t.artist, r_artists) if t.artist else 1.0
    return title_sc * 0.7 + artist_sc * 0.3


MATCH_THRESHOLD = 0.35


# ── 网易云搜索 + 下载 ──────────────────────────────────────

def ncm_search(query: str, limit: int = 8) -> list[dict]:
    if not HAS_PYNCM:
        return []
    try:
        r = cloudsearch.GetSearchResult(keyword=query, stype=1, limit=limit)
        return r.get("result", {}).get("songs", [])
    except Exception as e:
        print(f"  [网易云搜索错误] {e}")
        return []


def ncm_get_audio(song_id: int, bitrate: int) -> tuple[Optional[str], str, bool]:
    """返回 (url, ext, is_trial)"""
    try:
        r = ncm_track.GetTrackAudio([song_id], bitrate=bitrate)
        data = r.get("data", [])
        if data and data[0].get("url"):
            d = data[0]
            is_trial = bool(d.get("freeTrialInfo"))
            return d["url"], d.get("type", "mp3"), is_trial
    except Exception as e:
        print(f"  [获取链接错误] {e}")
    return None, "mp3", False


def ncm_download(t: Track, output_dir: Path, bitrate: int, authenticated: bool = False) -> bool:
    songs = ncm_search(t.query())
    if not songs:
        return False

    best = max(songs, key=lambda r: score_result(t, r))
    sc = score_result(t, best)
    if sc < MATCH_THRESHOLD:
        print(f"  → 网易云未找到可靠匹配 (最高分 {sc:.2f})")
        return False

    song_id   = best["id"]
    ar_names  = "/".join(a["name"] for a in best.get("ar", []))
    song_name = best.get("name", "")
    print(f"  → 网易云: {ar_names} - {song_name} (id:{song_id}, 匹配分:{sc:.2f})")

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
        return True
    except Exception as e:
        print(f"  → 下载失败: {e}")
        if dest.exists():
            dest.unlink()
        return False


# ── yt-dlp 备用 ───────────────────────────────────────────

def ytdlp_download(t: Track, output_dir: Path, fmt: str) -> bool:
    print(f"  → yt-dlp 搜索 YouTube...")
    cmd = [
        "yt-dlp",
        "--extract-audio", "--audio-format", fmt, "--audio-quality", "0",
        "--output", str(output_dir / "%(uploader)s - %(title)s.%(ext)s"),
        "--no-playlist", "--quiet", "--no-warnings",
        f"ytsearch1:{t.label()}",
    ]
    return subprocess.run(cmd).returncode == 0


# ── 主流程 ────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="从 txt/csv 歌单批量下载音乐（网易云 + yt-dlp 备用）"
    )
    parser.add_argument("file", nargs="?", help="输入文件 (txt 或 csv)")
    parser.add_argument("-o", "--output",   default="downloads", help="输出目录 (默认: downloads)")
    parser.add_argument("-f", "--format",   default="mp3",
                        choices=["mp3", "flac", "m4a", "opus"],
                        help="yt-dlp 备用下载的音频格式 (默认: mp3)")
    parser.add_argument("-q", "--quality",  default="exhigh",
                        choices=list(QUALITY_BITRATE),
                        help="网易云音质: standard=128k / high=192k / exhigh=320k / lossless (默认: exhigh)")
    parser.add_argument("--settings",       help="settings.json 路径，用于读取网易云账号凭据")
    parser.add_argument("--verify-login",   action="store_true", help="仅测试网易云登录，不下载")
    parser.add_argument("--no-ncm",         action="store_true", help="不使用网易云")
    parser.add_argument("--no-ytdlp",       action="store_true", help="不使用 yt-dlp 备用")
    parser.add_argument("--delay",          type=float, default=1.0, help="每首间隔秒数 (默认: 1.0)")
    parser.add_argument("--dry-run",        action="store_true", help="只解析，不下载")
    args = parser.parse_args()

    if not HAS_PYNCM:
        print("警告: pyncm 未安装 → pip install pyncm\n")

    # 加载并尝试登录网易云账号
    authenticated = False
    if HAS_PYNCM and not args.no_ncm:
        phone, password, cookie = load_ncm_credentials(args.settings)
        if phone or cookie:
            authenticated = ncm_authenticate(phone, password, cookie)
        else:
            print("[网易云] 未配置账号，以游客模式下载（VIP歌曲不可用）")

    # --verify-login 模式：只测试登录，不下载
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

    ncm_label = ""
    if not args.no_ncm:
        ncm_label = f"网易云({'已登录' if authenticated else '游客'})"
    ytdlp_label = " + yt-dlp备用" if not args.no_ytdlp else ""
    print(f"解析到 {len(tracks)} 首  |  音源: {ncm_label}{ytdlp_label}  |  输出: {output_dir}\n")

    if args.dry_run:
        for i, t in enumerate(tracks, 1):
            print(f"  {i:>3}. {t.label()}")
        return

    failed: list[str] = []

    for i, t in enumerate(tracks, 1):
        print(f"[{i}/{len(tracks)}] {t.label()}")
        ok = False

        if not args.no_ncm and HAS_PYNCM:
            ok = ncm_download(t, output_dir, bitrate, authenticated=authenticated)

        if not ok and not args.no_ytdlp:
            ok = ytdlp_download(t, output_dir, args.format)

        print("  ✓ 完成" if ok else "  ✗ 失败")
        if not ok:
            failed.append(t.label())

        if i < len(tracks):
            time.sleep(args.delay)

    total = len(tracks)
    print(f"\n完成 {total - len(failed)}/{total} 首")

    if failed:
        failed_path = output_dir / "failed.txt"
        failed_path.write_text("\n".join(failed), encoding="utf-8")
        print(f"失败 {len(failed)} 首，已写入 {failed_path}：")
        for f in failed:
            print(f"  - {f}")


if __name__ == "__main__":
    main()
