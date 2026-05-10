use axum::body::Body;
use axum::response::Response;
use http::header;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

/// Channel buffer size between the streaming task and the HTTP body writer.
///
/// Bounded so that when hyper stops draining (because the underlying TCP
/// connection is dead), our producer task blocks on `send` and we can detect
/// the dead client via `send_timeout`. The unbounded variant masked this
/// because the producer never blocked and `tx.is_closed()` was never updated
/// by hyper for connections that went idle before the first write.
pub const STREAM_CHANNEL_CAPACITY: usize = 4;

/// Create a streaming HTTP response that sends audio/mpeg data.
///
/// Returns a tuple of (sender, response). The caller feeds audio/mpeg data
/// chunks into the sender; the sender is bounded so a dead client surfaces
/// as `send` blocking.
pub fn create_stream_response() -> (mpsc::Sender<bytes::Bytes>, Response<Body>) {
    let (tx, rx) = mpsc::channel::<bytes::Bytes>(STREAM_CHANNEL_CAPACITY);
    let stream = ReceiverStream::new(rx)
        .map(|chunk: bytes::Bytes| Ok::<_, std::convert::Infallible>(chunk));

    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::CONNECTION, "keep-alive")
        .header("Server", "Rakuraku-Radio/3.0")
        .body(body)
        .expect("Failed to build streaming response — header values are static");

    (tx, response)
}
