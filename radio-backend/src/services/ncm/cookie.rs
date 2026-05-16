use anyhow::{Context, Result};

pub fn cookie_value(cookie: &str, name: &str) -> Option<String> {
    let prefix = format!("{}=", name);
    cookie.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&prefix).map(|v| v.to_string())
    })
}

pub fn has_cookie(cookie: &str, name: &str) -> bool {
    cookie_value(cookie, name)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}

pub fn validate_login_cookie(cookie: &str) -> Result<String> {
    let trimmed = cookie.trim();
    if trimmed.is_empty() {
        anyhow::bail!("请粘贴完整网易云 Cookie");
    }
    if !has_cookie(trimmed, "MUSIC_U") {
        anyhow::bail!("Cookie 缺少 MUSIC_U，请从已登录的 music.163.com 请求头复制完整 Cookie");
    }
    Ok(trimmed.to_string())
}

pub fn read_admin_cookie_from_secrets(path: &std::path::Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content =
        std::fs::read_to_string(path).with_context(|| format!("无法读取 {}", path.display()))?;
    let json: serde_json::Value =
        serde_json::from_str(&content).with_context(|| format!("无法解析 {}", path.display()))?;
    Ok(json
        .get("ncm_cookie")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned))
}
