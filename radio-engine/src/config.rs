/// Buffer capacity in bytes (512KB, must be power of 2)
pub const BUFFER_CAPACITY: usize = 524288; // 512 * 1024
/// Audio chunk size for ffmpeg reads
pub const AUDIO_CHUNK_SIZE: usize = 16384;
/// Epoll/timeout equivalent for clients (ms)
pub const STREAM_PUSH_INTERVAL_MS: u64 = 100;
/// Max concurrent streaming clients
pub const MAX_CONNECTIONS: usize = 1024;
/// Crossfade preload window (seconds before track end)
pub const CROSSFADE_SECONDS: u64 = 3;
/// How often to publish playback state (ms)
pub const STATE_PUBLISH_INTERVAL_MS: u64 = 500;
/// Default audio stream port
pub const DEFAULT_STREAM_PORT: u16 = 2240;
/// Supported audio formats (lowercase extensions)
pub const SUPPORTED_FORMATS: &[&str] = &["mp3", "wav", "flac", "ogg", "m4a", "aac"];
/// MP3 bitrate for ffmpeg transcoding
pub const MP3_BITRATE: &str = "128k";
/// Audio sample rate
pub const SAMPLE_RATE: u32 = 44100;
/// Audio channels
pub const CHANNELS: u8 = 2;
