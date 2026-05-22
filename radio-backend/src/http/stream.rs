//! HTTP audio streaming endpoint backed by the embedded engine ring buffer.

use crate::app::state::AppState;
use axum::extract::State;
use std::sync::Arc;

/// GET /stream — 音频流端点，从环形缓冲区广播音频数据
pub async fn stream_handler(State(state): State<Arc<AppState>>) -> axum::response::Response {
    use radio_engine::config::AUDIO_CHUNK_SIZE;
    use std::time::{Duration, Instant};

    const SEND_TIMEOUT: Duration = Duration::from_secs(5);
    const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
    const WAIT_DATA_MS: u64 = 500;

    let (tx, response) = radio_engine::stream::create_stream_response();
    let buffer = state.ring_buffer.clone();

    tokio::spawn(async move {
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

    response
}
