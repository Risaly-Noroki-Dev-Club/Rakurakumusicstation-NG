/// LRC 歌词解析器和时间偏移匹配器。
///
/// LRC 格式：
/// ```
/// [mm:ss.xx]歌词行文本
/// [mm:ss.xx]另一行
/// [mm:ss]不带百分秒
/// ```
/// 如 `[ti:标题]` `[ar:艺术家]` `[al:专辑]` `[offset:+/-毫秒]` 之类的标签将被跳过。

use once_cell::sync::Lazy;
use regex::Regex;

/// 单条 LRC 时间戳行。
#[derive(Debug, Clone)]
pub struct LyricLine {
    /// 从歌曲开始计算的毫秒时间戳。
    pub time_ms: i64,
    /// 此行的文本（纯器乐间奏时为空）。
    pub text: String,
}

/// 解析后的 LRC 文件，包含有序的时间戳行列表。
#[derive(Debug, Clone)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    /// 应用于所有时间戳的可选偏移量（来自 [offset:...] 标签）。
    pub offset_ms: i64,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// 编译后的正则表达式，用于匹配 LRC 时间戳：[mm:ss.xx] 或 [mm:ss]
static LRC_TIMESTAMP_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[(\d{1,3}):(\d{1,2})(?:\.(\d{1,3}))?\]").unwrap()
});

/// 编译后的正则表达式，用于匹配 LRC 中常见的类 ID3 标签。
static LRC_TAG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\[(ti|ar|al|offset):(.*)\]$").unwrap()
});

impl Lyrics {
    /// 从字符串内容解析 LRC 文件。
    pub fn parse(content: &str) -> Self {
        let mut lines: Vec<LyricLine> = Vec::new();
        let mut offset_ms: i64 = 0;
        let mut title = None;
        let mut artist = None;
        let mut album = None;

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            // 检查标签行：[ti:...]、[ar:...]、[al:...]、[offset:...]
            if let Some(caps) = LRC_TAG_RE.captures(line) {
                let tag = caps.get(1).unwrap().as_str();
                let value = caps.get(2).unwrap().as_str().trim().to_string();
                match tag {
                    "ti" => title = Some(value),
                    "ar" => artist = Some(value),
                    "al" => album = Some(value),
                    "offset" => {
                        // offset 以毫秒为单位，可以为负数
                        if let Ok(ms) = value.parse::<i64>() {
                            offset_ms = ms;
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // 解析此行中的所有时间戳（LRC 允许每行多个）
            let mut timestamps: Vec<i64> = Vec::new();
            let mut search_start = 0;
            let _line_bytes = line.as_bytes();

            loop {
                if search_start >= line.len() {
                    break;
                }

                if let Some(caps) = LRC_TIMESTAMP_RE.find(&line[search_start..]) {
                    let full_match = caps.as_str();
                    let _cap_start = search_start + caps.start();
                    let cap_end = search_start + caps.end();

                    // 解析捕获的时间戳
                    if let Some(inner_caps) = LRC_TIMESTAMP_RE.captures(full_match) {
                        let min: i64 = inner_caps.get(1).unwrap().as_str().parse().unwrap_or(0);
                        let sec: i64 = inner_caps.get(2).unwrap().as_str().parse().unwrap_or(0);
                        let ms: i64 = inner_caps
                            .get(3)
                            .map(|m| {
                                let s = m.as_str();
                                // 如果需要，补齐到 3 位（例如 "5" -> 500ms，"05" -> 50ms）
                                let padded = format!("{:0<3}", s);
                                padded.parse().unwrap_or(0)
                            })
                            .unwrap_or(0);

                        let time_ms = min * 60000 + sec * 1000 + ms + offset_ms;
                        timestamps.push(time_ms.max(0)); // 将负数限制为 0
                    }

                    search_start = cap_end;
                } else {
                    break;
                }
            }

            // 最后一个括号后的剩余文本即为歌词文本
            if !timestamps.is_empty() {
                let text_start = line
                    .rfind(']')
                    .map(|pos| pos + 1)
                    .unwrap_or(0);
                let text = line[text_start..].trim().to_string();

                for time_ms in timestamps {
                    lines.push(LyricLine {
                        time_ms,
                        text: text.clone(),
                    });
                }
            }
        }

        // 按时间戳排序（对多时间戳行很重要）
        lines.sort_by_key(|l| l.time_ms);

        // 合并相同时间戳的行（保留最后一条的文本）
        let mut merged: Vec<LyricLine> = Vec::new();
        for line in lines {
            if let Some(last) = merged.last_mut() {
                if last.time_ms == line.time_ms {
                    last.text = line.text;
                    continue;
                }
            }
            merged.push(line);
        }

        Lyrics {
            lines: merged,
            offset_ms,
            title,
            artist,
            album,
        }
    }

    /// 根据当前播放位置（毫秒）查找当前歌词行索引。
    /// 返回时间戳 <= position_ms 的最后一行索引。
    /// 如果尚未到达任何行，则返回 None。
    pub fn line_at(&self, position_ms: i64) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }

        // 二分查找以提高效率
        match self.lines.binary_search_by(|line| line.time_ms.cmp(&position_ms)) {
            Ok(idx) => Some(idx),
            Err(0) => None, // 位置在第一行之前
            Err(idx) => Some(idx - 1),
        }
    }

    /// 获取给定行索引的完整文本。
    pub fn line_text(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(|l| l.text.as_str())
    }

    /// 获取歌词行的总数。
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_lrc() {
        let lrc = r#"
[00:05.00]Line one
[00:10.00]Line two
[00:15.50]Line three
"#;
        let lyrics = Lyrics::parse(lrc);
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].time_ms, 5000);
        assert_eq!(lyrics.lines[1].time_ms, 10000);
        assert_eq!(lyrics.lines[2].time_ms, 15500);

        // 行查询
        assert_eq!(lyrics.line_at(0), None);
        assert_eq!(lyrics.line_at(5000), Some(0));
        assert_eq!(lyrics.line_at(7500), Some(0));
        assert_eq!(lyrics.line_at(10000), Some(1));
        assert_eq!(lyrics.line_at(20000), Some(2));
    }

    #[test]
    fn test_multiple_timestamps_per_line() {
        let lrc = "[00:05.00][00:10.00]Repeated line\n";
        let lyrics = Lyrics::parse(lrc);
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.lines[0].time_ms, 5000);
        assert_eq!(lyrics.lines[1].time_ms, 10000);
    }

    #[test]
    fn test_offset_tag() {
        let lrc = "[offset:+500]\n[00:05.00]Offset line\n";
        let lyrics = Lyrics::parse(lrc);
        assert_eq!(lyrics.lines[0].time_ms, 5500);
    }
}
