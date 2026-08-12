//! HTTP audio streaming endpoint backed by the embedded engine ring buffer.

use crate::app::state::AppState;
use crate::error::AppError;
use axum::extract::State;
use radio_engine::config::MAX_CONNECTIONS;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 当前 /stream 连接数（进程内）。
///
/// 与 WebSocket 的 WS_CONNECTIONS 同一套模式：连接洪泛时直接 429，
/// 防止 tokio task / 环形缓冲 reader / fd 无界增长拖垮进程。
static STREAM_CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// RAII：stream 任务结束时释放连接槽位（覆盖所有提前 return 路径）。
struct StreamSlot;

impl StreamSlot {
    fn acquire() -> Result<Self, AppError> {
        if STREAM_CONNECTIONS.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
            STREAM_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
            return Err(AppError::RateLimited(
                "Too many stream connections".into(),
            ));
        }
        Ok(Self)
    }
}

impl Drop for StreamSlot {
    fn drop(&mut self) {
        STREAM_CONNECTIONS.fetch_sub(1, Ordering::Relaxed);
    }
}

/// GET /stream — 音频流端点，从环形缓冲区广播音频数据
pub async fn stream_handler(
    State(state): State<Arc<AppState>>,
) -> Result<axum::response::Response, AppError> {
    use radio_engine::config::AUDIO_CHUNK_SIZE;
    use std::time::{Duration, Instant};

    const SEND_TIMEOUT: Duration = Duration::from_secs(5);
    /// 无数据可发时的保留时长。超过即关闭连接，让客户端重连到最新进度。
    /// 换曲/重启 ffmpeg 的静默间隙 <2s，15s 是充足余量；较旧的 60s 能更快
    /// 回收"服务端静默但连接存活"的槽位（配合前端停滞看门狗快速恢复）。
    const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
    const WAIT_DATA_MS: u64 = 500;

    let slot = StreamSlot::acquire()?;
    let (tx, response) = radio_engine::stream::create_stream_response();
    let buffer = state.ring_buffer.clone();

    tokio::spawn(async move {
        // 连接槽位随任务结束释放（RAII，覆盖所有 break 路径）。
        let _slot = slot;

        let reader = buffer.create_reader();
        let mut buf = vec![0u8; AUDIO_CHUNK_SIZE];
        let mut last_progress = Instant::now();

        loop {
            if tx.is_closed() {
                break;
            }
            if last_progress.elapsed() > IDLE_TIMEOUT {
                tracing::debug!("Stream idle timeout, closing");
                break;
            }

            let (available, should_resync) = reader.wait_for_data_or_resync(WAIT_DATA_MS).await;
            if should_resync {
                tracing::debug!("Stream resync requested, closing client response");
                break;
            }
            if available == 0 {
                continue;
            }

            let to_read = std::cmp::min(buf.len(), available);
            let n = reader.read(&mut buf[..to_read]);
            if n == 0 {
                continue;
            }

            let chunk = bytes::Bytes::copy_from_slice(&buf[..n]);
            match tokio::time::timeout(SEND_TIMEOUT, tx.send(chunk)).await {
                Ok(Ok(())) => {
                    last_progress = Instant::now();
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    tracing::debug!("Stream send timeout — client likely dead");
                    break;
                }
            }
        }

        tracing::debug!("Stream client disconnected, reader cleaned up");
    });

    Ok(response)
}
