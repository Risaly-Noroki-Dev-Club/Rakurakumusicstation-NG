//! Shared application state passed to Axum handlers.

use crate::config::{AppConfig, StationConfig};
use radio_engine::player::PlayerHandle;
use radio_engine::ring_buffer::RingBuffer;
use sqlx::SqlitePool;
use std::sync::{Arc, RwLock};

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
        })
    }
}
