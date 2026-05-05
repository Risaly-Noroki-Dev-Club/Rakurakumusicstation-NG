/// 数据库初始化、连接池和迁移。

use crate::config::DatabaseConfig;
use crate::config::StationConfig;
use radio_engine::ring_buffer::RingBuffer;
use radio_engine::player::PlayerHandle;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

/// 所有请求处理器共享的应用状态。
pub struct AppState {
    pub db: SqlitePool,
    pub config: crate::config::AppConfig,
    pub station: RwLock<StationConfig>,
    pub ws_tx: tokio::sync::broadcast::Sender<String>,
    /// 音频引擎的环形缓冲区（用于流式传输）
    pub ring_buffer: Arc<RingBuffer>,
    /// 音频引擎的播放器句柄（用于发送命令、获取状态）
    pub player_handle: PlayerHandle,
}

impl AppState {
    /// 创建包含所有已初始化组件的新 AppState。
    pub async fn new(
        config: crate::config::AppConfig,
        ring_buffer: Arc<RingBuffer>,
        player_handle: PlayerHandle,
    ) -> anyhow::Result<Self> {
        let db = init_database(&config.database).await?;

        let (ws_tx, _) = tokio::sync::broadcast::channel(1024);

        let station = RwLock::new(config.station.clone());

        Ok(Self {
            db,
            config,
            station,
            ws_tx,
            ring_buffer,
            player_handle,
        })
    }
}

/// 初始化 SQLite 数据库连接池并运行迁移。
async fn init_database(config: &DatabaseConfig) -> anyhow::Result<SqlitePool> {
    if config.url.starts_with("sqlite:") {
        if let Some(path) = config.url.strip_prefix("sqlite://") {
            if path.contains('/') {
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent)?;
                }
            }
        }
    }

    let options = SqliteConnectOptions::from_str(&config.url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    tracing::info!("Database initialized successfully");

    Ok(pool)
}
