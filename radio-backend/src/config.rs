/// 应用配置，从 config.toml 加载，支持通过环境变量覆盖。
/// 环境变量使用 `RADIO_` 前缀和大写路径表示法
/// （例如 `RADIO_SERVER__PORT` 覆盖 `[server] port`）。

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub audio_engine: AudioEngineConfig,
    pub jwt: JwtConfig,
    pub queue: QueueConfig,
    pub station: StationConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_sqlite_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    #[serde(default = "default_redis_url")]
    pub url: String,
    #[serde(default = "default_playback_channel")]
    pub playback_channel: String,
    #[serde(default = "default_command_channel")]
    pub command_channel: String,
    #[serde(default = "default_queue_channel")]
    pub queue_channel: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioEngineConfig {
    #[serde(default = "default_engine_base_url")]
    pub base_url: String,
    #[serde(default = "default_media_path")]
    pub media_path: String,
    /// 音频流 URL — 可以是绝对路径 (http://...) 或相对路径 (/stream)。
    /// 相对路径会与 base_url 拼接。
    #[serde(default = "default_stream_base")]
    pub stream_base: String,
}

impl AudioEngineConfig {
    /// 将 stream_base 解析为绝对 URL。
    /// - 如果 stream_base 以 http:// 或 https:// 开头 → 直接使用
    /// - 否则 → 拼接 base_url + stream_base
    pub fn resolve_stream_url(&self) -> String {
        if self.stream_base.starts_with("http://") || self.stream_base.starts_with("https://") {
            self.stream_base.clone()
        } else {
            let base = self.base_url.trim_end_matches('/');
            let path = self.stream_base.trim_start_matches('/');
            format!("{}/{}", base, path)
        }
    }

    /// 构建当前播放曲目的文件 URL。
    pub fn resolve_file_url(&self, song_id: i64) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/file/{}", base, song_id)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    #[serde(default = "default_jwt_secret")]
    pub secret: String,
    #[serde(default = "default_expiry_hours")]
    pub expiry_hours: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    #[serde(default = "default_max_queue_size")]
    pub max_size: usize,
    #[serde(default = "default_max_user_submissions")]
    pub max_user_submissions: usize,
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StationConfig {
    #[serde(default = "default_station_name")]
    pub name: String,
    #[serde(default = "default_subtitle")]
    pub subtitle: String,
    #[serde(default = "default_primary_color")]
    pub primary_color: String,
    #[serde(default = "default_secondary_color")]
    pub secondary_color: String,
    #[serde(default = "default_bg_color")]
    pub bg_color: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

// ─── 默认值 ─────────────────────────────────────────────

fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 2241 }
fn default_sqlite_url() -> String { "sqlite://data/radio.db?mode=rwc".into() }
fn default_redis_url() -> String { "redis://127.0.0.1:6379".into() }
fn default_playback_channel() -> String { "playback_state".into() }
fn default_command_channel() -> String { "command".into() }
fn default_queue_channel() -> String { "queue_event".into() }
fn default_engine_base_url() -> String { "http://127.0.0.1:2240".into() }
fn default_media_path() -> String { "./media".into() }
fn default_stream_base() -> String { "/stream".into() }
fn default_jwt_secret() -> String { "radio-backend-dev-secret-change-in-production".into() }
fn default_expiry_hours() -> u64 { 24 }
fn default_max_queue_size() -> usize { 100 }
fn default_max_user_submissions() -> usize { 3 }
fn default_rate_limit_window() -> u64 { 300 }
fn default_station_name() -> String { "Rakuraku Music Station".into() }
fn default_subtitle() -> String { "A Community Radio".into() }
fn default_primary_color() -> String { "#764ba2".into() }
fn default_secondary_color() -> String { "#667eea".into() }
fn default_bg_color() -> String { "#f4f4f9".into() }
fn default_log_level() -> String { "info".into() }

impl AppConfig {
    /// 从 TOML 文件加载配置，然后通过环境变量覆盖。
    /// 环境变量使用 `RADIO_` 前缀和双下划线分隔符
    /// 表示嵌套键，例如 `RADIO_SERVER__PORT=9090`。
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&content)?;

        // 通过环境变量覆盖
        if let Ok(v) = std::env::var("RADIO_DATABASE_URL") { config.database.url = v; }
        if let Ok(v) = std::env::var("RADIO_REDIS_URL") { config.redis.url = v; }
        if let Ok(v) = std::env::var("RADIO_JWT_SECRET") { config.jwt.secret = v; }
        if let Ok(v) = std::env::var("RADIO_SERVER_PORT") {
            config.server.port = v.parse().unwrap_or(config.server.port);
        }
        if let Ok(v) = std::env::var("RADIO_LOG_LEVEL") { config.logging.level = v; }
        if let Ok(v) = std::env::var("RADIO_MEDIA_PATH") { config.audio_engine.media_path = v; }
        if let Ok(v) = std::env::var("RADIO_STREAM_BASE") { config.audio_engine.stream_base = v; }
        if let Ok(v) = std::env::var("RADIO_STATION_NAME") { config.station.name = v; }

        Ok(config)
    }

    /// 便捷方法：从默认路径或环境变量构建配置。
    pub fn load_default() -> anyhow::Result<Self> {
        let path = std::env::var("RADIO_CONFIG")
            .unwrap_or_else(|_| "config.toml".to_string());
        Self::load(&path)
    }
}
