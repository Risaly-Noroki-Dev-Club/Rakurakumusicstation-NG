use super::crypto::{eapi_decrypt, eapi_encrypt};
use anyhow::Result;
use rand::Rng;
use reqwest::Client;
use std::time::Duration;

const NOBODY_KNOWS: &str = "36cd479b6b5";

fn generate_device_id() -> String {
    let mut rng = rand::thread_rng();
    let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
    (0..32)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[derive(Debug, Clone)]
pub struct NcmClient {
    pub device_id: String,
    pub music_u: Option<String>,
    http_client: Client,
}

impl NcmClient {
    pub fn new(device_id: Option<String>, music_u: Option<String>) -> Self {
        let device_id = device_id.unwrap_or_else(generate_device_id);
        let http_client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            device_id,
            music_u,
            http_client,
        }
    }

    fn build_cookie_header(&self) -> String {
        let mut cookies = vec![
            format!("deviceId={}", self.device_id),
            format!("appver=9.3.40"),
            format!(
                "buildver={}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
            format!("resolution=1920x1080"),
            format!("os=Android"),
        ];

        if let Some(mu) = &self.music_u {
            if !mu.is_empty() {
                cookies.push(format!("MUSIC_U={}", mu));
            } else {
                cookies.push(format!("MUSIC_A=4ee5f776c9ed1e4d5f031b09e084c6cb333e43ee4a841afeebbef9bbf4b7e4152b51ff20ecb9e8ee9e89ab23044cf50d1609e4781e805e73a138419e5583bc7fd1e5933c52368d9127ba9ce4e2f233bf5a77ba40ea6045ae1fc612ead95d7b0e0edf70a74334194e1a190979f5fc12e9968c3666a981495b33a649814e309366"));
            }
        } else {
            cookies.push(format!("MUSIC_A=4ee5f776c9ed1e4d5f031b09e084c6cb333e43ee4a841afeebbef9bbf4b7e4152b51ff20ecb9e8ee9e89ab23044cf50d1609e4781e805e73a138419e5583bc7fd1e5933c52368d9127ba9ce4e2f233bf5a77ba40ea6045ae1fc612ead95d7b0e0edf70a74334194e1a190979f5fc12e9968c3666a981495b33a649814e309366"));
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

    pub async fn eapi_request(&self, path: &str, url: &str, json_body: &str) -> Result<String> {
        let splice = Self::splice_str(path, json_body);
        let body = Self::format_params(&splice);

        let response = self
            .http_client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", Self::choose_user_agent())
            .header("Cookie", self.build_cookie_header())
            .body(body)
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;
        let decrypted = eapi_decrypt(&bytes);
        let text = String::from_utf8_lossy(&decrypted);
        let text = text.trim().to_string();

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

        let looks_like_json = text.starts_with('{') || text.starts_with('[');
        if !looks_like_json {
            let raw = String::from_utf8_lossy(&bytes);
            let excerpt = if text.chars().all(|c| c.is_control()) {
                raw.trim().chars().take(240).collect::<String>()
            } else {
                text.chars().take(240).collect::<String>()
            };
            anyhow::bail!(
                "网易云接口返回非 JSON 响应: {} ({})，可能是 Cookie 过期、触发风控或接口变更。响应片段: {}",
                path,
                status,
                excerpt.replace('\n', " "),
            );
        }

        Ok(text)
    }

    pub async fn test_login(&self) -> Result<bool> {
        let result = self
            .eapi_request(
                "/api/w/user/setting",
                "https://music.163.com/eapi/w/user/setting",
                "{}",
            )
            .await?;
        let json: serde_json::Value = serde_json::from_str(&result)?;
        Ok(json.get("code").and_then(|c| c.as_i64()) == Some(200))
    }
}
