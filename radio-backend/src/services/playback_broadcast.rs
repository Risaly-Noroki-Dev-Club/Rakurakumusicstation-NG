//! Periodically reads engine playback state and broadcasts WebSocket updates.

use crate::app::state::AppState;
use crate::services::playback_snapshot::PlaybackSnapshotCache;
use std::sync::Arc;

/// 启动引擎状态轮询器，将播放状态转发给 WebSocket 客户端。
/// 直接读取内嵌引擎的状态（不通过 HTTP）。
///
/// ## 内存优化
/// - 仅在有订阅者时才发送消息
/// - 歌词仅在切换歌曲时解析一次并缓存，且只在首条消息中发送全量歌词
/// - 后续消息只发送当前歌词行索引，避免每 500ms 克隆数十 KB 的歌词数组
pub fn start_engine_state_poller(state: Arc<AppState>) {
    let state_clone = state.clone();

    tokio::spawn(async move {
        let mut snapshot_cache = PlaybackSnapshotCache::new();

        tracing::info!("Engine state poller started");

        loop {
            let ps = state_clone.player_handle.get_state();

            // 仅在有活跃订阅者时才发送消息
            if state_clone.ws_tx.receiver_count() > 0 {
                let enriched = snapshot_cache.build_message(&state_clone, &ps).await;

                let _ = state_clone
                    .ws_tx
                    .send(serde_json::to_string(&enriched).unwrap_or_default());
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
}
