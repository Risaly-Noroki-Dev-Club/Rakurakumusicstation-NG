//! Shared application state passed to Axum handlers.

use crate::config::{AppConfig, StationConfig};
use dashmap::DashMap;
use radio_engine::player::PlayerHandle;
use radio_engine::ring_buffer::RingBuffer;
use sqlx::SqlitePool;
use std::sync::{Arc, RwLock};

/// 在线听众信息
#[derive(Debug, Clone)]
pub struct OnlineListener {
    pub display_name: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

/// 所有请求处理器共享的应用状态。
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub station: RwLock<StationConfig>,
    pub ws_tx: tokio::sync::broadcast::Sender<String>,
    /// 音频引擎的环形缓冲区（用于流式传输）
    pub ring_buffer: Arc<RingBuffer>,
    /// 音频引擎的播放器句柄（用于发送命令、获取状态）
    pub player_handle: PlayerHandle,
    /// Serializes DB queue mutations with embedded-engine request queue updates.
    pub queue_sync: tokio::sync::Mutex<()>,
    /// 在线听众注册表 (device_token -> OnlineListener)
    pub listeners: Arc<DashMap<String, OnlineListener>>,
}

impl AppState {
    /// 创建包含所有已初始化组件的新 AppState。
    pub async fn new(
        config: AppConfig,
        ring_buffer: Arc<RingBuffer>,
        player_handle: PlayerHandle,
    ) -> anyhow::Result<Self> {
        let db = crate::db::init_database(&config.database).await?;
        let (ws_tx, _) = tokio::sync::broadcast::channel(1024);
        let station = RwLock::new(config.station.clone());

        Ok(Self {
            db,
            config,
            station,
            ws_tx,
            ring_buffer,
            player_handle,
            queue_sync: tokio::sync::Mutex::new(()),
            listeners: Arc::new(DashMap::new()),
        })
    }
}
