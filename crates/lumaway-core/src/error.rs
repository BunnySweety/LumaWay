pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("GStreamer error: {0}")]
    GStreamer(String),

    #[error("GStreamer element not found: {0}")]
    MissingElement(&'static str),

    #[error("capture timed out waiting for a frame")]
    CaptureTimeout,

    #[error("portal error: {0}")]
    Portal(String),

    #[error("failed to access {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
}
