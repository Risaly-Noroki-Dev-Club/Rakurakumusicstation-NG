use super::cookie::cookie_value;
use super::crypto::{eapi_decrypt, eapi_encrypt};
use anyhow::Result;
use rand::Rng;
use reqwest::Client;
use std::time::Duration;

const NOBODY_KNOWS: &str = "36cd479b6b5";

fn generate_device_id() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "0123456789abcdef".chars().collect();
    (0..32)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[derive(Debug, Clone)]
pub struct NcmClient {
    pub device_id: String,
    pub cookie: Option<String>,
    http_client: Client,
}

impl NcmClient {
    pub fn new(device_id: Option<String>, cookie: Option<String>) -> Self {
        let device_id = device_id.unwrap_or_else(generate_device_id);
        let http_client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            device_id,
            cookie,
            http_client,
        }
    }

    pub(crate) fn build_cookie_header(&self) -> String {
        let mut cookies: Vec<String> = self
            .cookie
            .as_deref()
            .unwrap_or("")
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        let has = |items: &[String], name: &str| {
            let prefix = format!("{}=", name);
            items.iter().any(|c| c.starts_with(&prefix))
        };

        if !has(&cookies, "deviceId") {
            cookies.push(format!("deviceId={}", self.device_id));
        }
        if !has(&cookies, "appver") {
            cookies.push("appver=9.3.40".to_string());
        }
        if !has(&cookies, "buildver") {
            cookies.push(format!(
                "buildver={}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ));
        }
        if !has(&cookies, "resolution") {
            cookies.push("resolution=1920x1080".to_string());
        }
        if !has(&cookies, "os") {
            cookies.push("os=android".to_string());
        }
        cookies.join("; ")
    }

    fn choose_user_agent() -> &'static str {
        "Mozilla/5.0 (iPhone; CPU iPhone OS 10_0 like Mac OS X) AppleWebKit/602.1.38 (KHTML, like Gecko) Version/10.0 Mobile/14A300 Safari/602.1"
    }

    fn splice_str(path: &str, data: &str) -> String {
        use md5::{Digest, Md5};
        let text = format!("nobody{}use{}md5forencrypt", path, data);
        let mut hasher = Md5::new();
        hasher.update(text.as_bytes());
        let result = hasher.finalize();
        let md5_hex = format!("{:x}", result);
        format!(
            "{}-{}-{}-{}-{}",
            path, NOBODY_KNOWS, data, NOBODY_KNOWS, md5_hex
        )
    }

    fn format_params(splice: &str) -> String {
        let encrypted = eapi_encrypt(splice);
        format!("params={}", hex::encode_upper(&encrypted))
    }

    fn eapi_header(&self) -> Option<serde_json::Map<String, serde_json::Value>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let cookie = self.cookie.as_deref().unwrap_or_default();
        if !["MUSIC_U", "MUSIC_A"]
            .iter()
            .any(|name| cookie_value(cookie, name).is_some())
        {
            return None;
        }
        let csrf = cookie_value(cookie, "__csrf").unwrap_or_default();
        let mut header = serde_json::Map::from_iter([
            ("osver".into(), serde_json::Value::String("".into())),
            (
                "deviceId".into(),
                serde_json::Value::String(self.device_id.clone()),
            ),
            ("appver".into(), serde_json::Value::String("9.3.40".into())),
            (
                "versioncode".into(),
                serde_json::Value::String("140".into()),
            ),
            (
                "buildver".into(),
                serde_json::Value::String(now.as_secs().to_string()),
            ),
            (
                "resolution".into(),
                serde_json::Value::String("1920x1080".into()),
            ),
            ("__csrf".into(), serde_json::Value::String(csrf)),
            ("os".into(), serde_json::Value::String("android".into())),
            (
                "requestId".into(),
                serde_json::Value::String(format!("{}_0000", now.as_millis())),
            ),
        ]);
        for name in ["MUSIC_U", "MUSIC_A"] {
            if let Some(value) = cookie_value(cookie, name) {
                header.insert(name.into(), serde_json::Value::String(value));
            }
        }
        Some(header)
    }

    pub async fn eapi_request(&self, path: &str, url: &str, json_body: &str) -> Result<String> {
        let mut body_json: serde_json::Value =
            serde_json::from_str(json_body).unwrap_or_else(|_| serde_json::json!({}));
        if let Some(csrf) = self
            .cookie
            .as_deref()
            .and_then(|c| cookie_value(c, "__csrf"))
        {
            if let Some(map) = body_json.as_object_mut() {
                map.entry("csrf_token".to_string())
                    .or_insert(serde_json::Value::String(csrf));
            }
        }
        if let (Some(map), Some(header)) = (body_json.as_object_mut(), self.eapi_header()) {
            map.insert("header".to_string(), serde_json::Value::Object(header));
        }
        let json_body = body_json.to_string();
        let splice = Self::splice_str(path, &json_body);
        let body = Self::format_params(&splice);

        let response = self
            .http_client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "*/*")
            .header("Accept-Language", "zh-CN,zh;q=0.9,en;q=0.8")
            .header("Origin", "https://music.163.com")
            .header("Referer", "https://music.163.com/")
            .header("User-Agent", Self::choose_user_agent())
            .header("Cookie", self.build_cookie_header())
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;
        let text = decode_eapi_response(&bytes)?;

        if !status.is_success() {
            anyhow::bail!("Eapi request failed: HTTP {} -> {}", status, text);
        }

        if text.is_empty() {
            anyhow::bail!(
                "网易云接口返回空响应: {} ({})，可能是 Cookie 过期、触发风控或接口格式变化",
                path,
                status,
            );
        }

        Ok(text)
    }

    pub async fn test_login(&self) -> Result<bool> {
        let response = self
            .http_client
            .post("https://music.163.com/api/w/nuser/account/get")
            .header("Accept", "application/json")
            .header("Referer", "https://music.163.com/")
            .header("User-Agent", Self::choose_user_agent())
            .header("Cookie", self.build_cookie_header())
            .send()
            .await?;
        let status = response.status();
        let json: serde_json::Value = response.json().await?;
        if !status.is_success() {
            anyhow::bail!("网易云登录状态接口返回 HTTP {}", status);
        }
        if json.get("code").and_then(|value| value.as_i64()) != Some(200) {
            anyhow::bail!("网易云登录状态接口返回 code={:?}", json.get("code"));
        }
        Ok(account_response_is_logged_in(&json))
    }
}

fn decode_eapi_response(bytes: &[u8]) -> Result<String> {
    if let Ok(raw) = std::str::from_utf8(bytes) {
        let raw = raw.trim();
        if !raw.is_empty() && serde_json::from_str::<serde_json::Value>(raw).is_ok() {
            return Ok(raw.to_string());
        }
    }

    if !bytes.is_empty() && bytes.len().is_multiple_of(16) {
        let decrypted = eapi_decrypt(bytes);
        if let Ok(text) = String::from_utf8(decrypted) {
            let text = text.trim();
            if !text.is_empty() && serde_json::from_str::<serde_json::Value>(text).is_ok() {
                return Ok(text.to_string());
            }
        }
    }

    let excerpt = String::from_utf8_lossy(bytes)
        .trim()
        .chars()
        .take(240)
        .collect::<String>()
        .replace('\n', " ");
    anyhow::bail!(
        "网易云接口返回非 JSON 响应，可能是 Cookie 过期、触发风控或接口变更。响应片段: {}",
        excerpt
    )
}

fn account_response_is_logged_in(json: &serde_json::Value) -> bool {
    json.get("code").and_then(|value| value.as_i64()) == Some(200)
        && ["account", "profile"].iter().any(|key| {
            json.get(*key)
                .is_some_and(|value| !value.is_null() && value.is_object())
        })
}

#[cfg(test)]
mod tests {
    use super::{account_response_is_logged_in, decode_eapi_response, NcmClient};
    use crate::services::ncm::crypto::eapi_encrypt;

    #[test]
    fn accepts_plaintext_eapi_json() {
        let json = br#"{"code":200,"data":[]}"#;
        assert_eq!(
            decode_eapi_response(json).unwrap(),
            String::from_utf8_lossy(json)
        );
    }

    #[test]
    fn accepts_encrypted_eapi_json() {
        let json = r#"{"code":200,"data":[]}"#;
        assert_eq!(decode_eapi_response(&eapi_encrypt(json)).unwrap(), json);
    }

    #[test]
    fn login_requires_a_real_account_or_profile() {
        assert!(!account_response_is_logged_in(&serde_json::json!({
            "code": 200,
            "account": null,
            "profile": null
        })));
        assert!(account_response_is_logged_in(&serde_json::json!({
            "code": 200,
            "account": { "id": 1 },
            "profile": null
        })));
    }

    #[test]
    fn eapi_mobile_header_requires_an_auth_token() {
        let anonymous = NcmClient::new(Some("0123456789abcdef0123456789abcdef".into()), None);
        assert!(anonymous.eapi_header().is_none());

        let authenticated = NcmClient::new(
            Some("0123456789abcdef0123456789abcdef".into()),
            Some("MUSIC_U=test-token; __csrf=test-csrf".into()),
        );
        let header = authenticated.eapi_header().unwrap();
        assert_eq!(
            header.get("MUSIC_U").and_then(|value| value.as_str()),
            Some("test-token")
        );
        assert_eq!(
            header.get("__csrf").and_then(|value| value.as_str()),
            Some("test-csrf")
        );
    }
}
