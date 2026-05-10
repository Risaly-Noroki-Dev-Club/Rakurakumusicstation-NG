use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetdiskFile {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub fs_id: i64,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct NetdiskShareInfo {
    pub surl: String,
    pub pwd: Option<String>,
    pub shareid: String,
    pub uk: String,
    pub bdstoken: Option<String>,
    pub seckey: Option<String>,
}

fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

fn parse_surl_from_url(url: &str) -> Option<String> {
    // https://pan.baidu.com/s/1xxxxx
    if let Some(caps) = Regex::new(r"pan\.baidu\.com/s/1([a-zA-Z0-9_-]+)").unwrap().captures(url) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    // https://pan.baidu.com/share/init?surl=xxxxx
    if let Some(caps) = Regex::new(r"[?&]surl=([a-zA-Z0-9_-]+)").unwrap().captures(url) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    None
}

fn parse_pwd_from_url(url: &str) -> Option<String> {
    // &pwd=xxxx or &password=xxxx or #/path?pwd=xxxx
    if let Some(caps) = Regex::new(r"[?&#&]pwd=([a-zA-Z0-9]{4})").unwrap().captures(url) {
        return Some(caps.get(1)?.as_str().to_string());
    }
    None
}

async fn fetch_share_page(surl: &str, pwd: Option<&str>) -> Result<String> {
    let url = format!("https://pan.baidu.com/s/1{}", surl);
    let client = http_client();
    let resp = client.get(&url).header("User-Agent", "Mozilla/5.0").send().await?;
    let mut html = resp.text().await?;

    // Check if password required
    if html.contains("init") && html.contains("verify") || html.contains("请输入提取码") {
        if let Some(p) = pwd {
            // Get bdstoken from init page
            let bdstoken = extract_bdstoken(&html);
            let url = format!("https://pan.baidu.com/share/verify?surl={}&bdstoken={}&channel=chunlei&clienttype=0&web=1&app_id=250528", surl, bdstoken.unwrap_or_default());
            let mut params = HashMap::new();
            params.insert("pwd".to_string(), p.to_string());
            params.insert("vcode".to_string(), "".to_string());
            params.insert("vcode_str".to_string(), "".to_string());

            let verify_resp = client.post(&url)
                .header("User-Agent", "Mozilla/5.0")
                .header("Referer", format!("https://pan.baidu.com/s/1{}", surl))
                .header("X-Requested-With", "XMLHttpRequest")
                .form(&params)
                .send().await?;

            let verify_json: serde_json::Value = verify_resp.json().await?;
            if verify_json.get("errno").and_then(|v| v.as_i64()) != Some(0) {
                return Err(anyhow!("提取码错误"));
            }

            // Fetch again with cookies
            let resp = client.get(format!("https://pan.baidu.com/s/1{}", surl))
                .header("User-Agent", "Mozilla/5.0")
                .send().await?;
            html = resp.text().await?;
        } else {
            return Err(anyhow!("该分享需要提取码"));
        }
    }

    Ok(html)
}

fn extract_bdstoken(html: &str) -> Option<String> {
    let re = Regex::new(r#"bdstoken["']?\s*[:=]\s*["']?([a-f0-9]{32})"#).ok()?;
    re.captures(html).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

fn extract_share_data(html: &str) -> Result<(String, String, Option<String>, Option<String>)> {
    // Try to extract from yunData or locals
    let re_shareid = Regex::new(r#"shareid["']?\s*[:=]\s*['"]?([0-9]+)"#).unwrap();
    let re_uk = Regex::new(r#"uk["']?\s*[:=]\s*['"]?([0-9]+)"#).unwrap();
    let re_seckey = Regex::new(r#"seckey["']?\s*[:=]\s*['"]([^'"]+)['"]"#).unwrap();

    let shareid = re_shareid.captures(html)
        .and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("无法解析 shareid"))?;

    let uk = re_uk.captures(html)
        .and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
        .ok_or_else(|| anyhow!("无法解析 uk"))?;

    let bdstoken = extract_bdstoken(html);
    let seckey = re_seckey.captures(html)
        .and_then(|c| c.get(1)).map(|m| m.as_str().to_string());

    Ok((shareid, uk, bdstoken, seckey))
}

pub async fn get_share_info(url: &str) -> Result<NetdiskShareInfo> {
    let surl = parse_surl_from_url(url)
        .ok_or_else(|| anyhow!("无法解析百度网盘分享链接"))?;
    let pwd = parse_pwd_from_url(url);

    let html = fetch_share_page(&surl, pwd.as_deref()).await?;
    let (shareid, uk, bdstoken, seckey) = extract_share_data(&html)?;

    Ok(NetdiskShareInfo {
        surl,
        pwd,
        shareid,
        uk,
        bdstoken,
        seckey,
    })
}

pub async fn list_share_files(info: &NetdiskShareInfo) -> Result<Vec<NetdiskFile>> {
    let client = http_client();
    let url = format!(
        "https://pan.baidu.com/share/list?shareid={}&uk={}&dir=/&page=1&num=1000&order=time&desc=1&clienttype=0&web=1&app_id=250528",
        info.shareid, info.uk
    );

    let mut req = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", format!("https://pan.baidu.com/s/1{}", info.surl));

    // Add cookies if we have them
    req = req.header("Cookie", format!("BDUSS=; PANWEB=1;"));

    let resp = req.send().await?;
    let json: serde_json::Value = resp.json().await?;

    let errno = json.get("errno").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errno != 0 {
        return Err(anyhow!("获取文件列表失败: errno={}", errno));
    }

    let list = json.get("list").and_then(|v| v.as_array()).ok_or_else(|| anyhow!("文件列表为空"))?;

    let mut files = Vec::new();
    for item in list {
        let path = item.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let filename = item.get("server_filename").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let size = item.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
        let fs_id = item.get("fs_id").and_then(|v| v.as_i64()).unwrap_or(0);
        let is_dir = item.get("isdir").and_then(|v| v.as_i64()).unwrap_or(0) == 1;

        files.push(NetdiskFile {
            path,
            filename,
            size,
            fs_id,
            is_dir,
        });
    }

    Ok(files)
}

pub async fn get_download_link(info: &NetdiskShareInfo, fs_id: i64) -> Result<String> {
    let client = http_client();

    // First get sign and timestamp from share page
    let url = format!("https://pan.baidu.com/s/1{}", info.surl);
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send().await?;
    let html = resp.text().await?;

    let sign_re = Regex::new(r#"sign["']?\s*[:=]\s*['"]([a-f0-9]+)['"]"#).unwrap();
    let timestamp_re = Regex::new(r#"timestamp["']?\s*[:=]\s*([0-9]+)"#).unwrap();

    let sign = sign_re.captures(&html)
        .and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
        .unwrap_or_default();
    let timestamp = timestamp_re.captures(&html)
        .and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
        .unwrap_or_default();

    let download_url = "https://pan.baidu.com/api/sharedownload";
    let mut params = HashMap::new();
    params.insert("sign".to_string(), sign);
    params.insert("timestamp".to_string(), timestamp);
    params.insert("channel".to_string(), "chunlei".to_string());
    params.insert("clienttype".to_string(), "0".to_string());
    params.insert("web".to_string(), "1".to_string());
    params.insert("app_id".to_string(), "250528".to_string());
    params.insert("seckey".to_string(), info.seckey.clone().unwrap_or_default());
    params.insert("encrypt".to_string(), "0".to_string());
    params.insert("product".to_string(), "share".to_string());
    params.insert("uk".to_string(), info.uk.clone());
    params.insert("primaryid".to_string(), info.shareid.clone());
    params.insert("fid_list".to_string(), format!("[{}]", fs_id));

    let resp = client.post(download_url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .header("Referer", format!("https://pan.baidu.com/s/1{}", info.surl))
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&params)
        .send().await?;

    let json: serde_json::Value = resp.json().await?;
    let errno = json.get("errno").and_then(|v| v.as_i64()).unwrap_or(-1);
    if errno != 0 {
        return Err(anyhow!("获取下载链接失败: errno={}", errno));
    }

    let list = json.get("list").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("下载链接列表为空"))?;

    for item in list {
        if let Some(url) = item.get("dlink").and_then(|v| v.as_str()) {
            return Ok(url.to_string());
        }
    }

    Err(anyhow!("未找到下载链接"))
}

pub async fn download_file(url: &str, output_path: &std::path::Path) -> Result<u64> {
    let client = http_client();
    let resp = client.get(url)
        .header("User-Agent", "Mozilla/5.0")
        .header("Accept", "*/*")
        .send().await?;

    if !resp.status().is_success() {
        return Err(anyhow!("下载失败: HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().await?;
    let size = bytes.len() as u64;

    if let Some(parent) = output_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(output_path, &bytes).await?;

    Ok(size)
}