use axum::body::Body;
use axum::response::Response;
use http::header;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

/// Create a streaming HTTP response that sends audio/mpeg data.
///
/// Returns a tuple of (sender, response). The caller should feed audio/mpeg
/// data chunks into the sender, and return the response to the HTTP framework
/// (Axum). Each client gets its own mpsc channel pair.
///
/// The response includes appropriate headers for audio streaming:
/// - Content-Type: audio/mpeg
/// - Cache-Control: no-cache
/// - Access-Control-Allow-Origin: *
/// - Connection: keep-alive
pub fn create_stream_response() -> (mpsc::UnboundedSender<bytes::Bytes>, Response<Body>) {
    let (tx, rx) = mpsc::unbounded_channel::<bytes::Bytes>();
    let stream = UnboundedReceiverStream::new(rx)
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
