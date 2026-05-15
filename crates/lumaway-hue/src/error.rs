pub type Result<T> = std::result::Result<T, HueError>;

#[derive(Debug, thiserror::Error)]
pub enum HueError {
    #[error("bridge IP is required")]
    MissingBridgeIp,

    #[error("app key is required for this operation")]
    MissingAppKey,

    #[error("client key is required for Hue Entertainment DTLS")]
    MissingClientKey,

    #[error("Hue TLS certificate pinning: {0}")]
    TlsPin(String),

    #[error("Hue request failed: {0}")]
    Request(String),

    #[error("Hue bridge rejected the request: {0}")]
    Bridge(String),

    #[error("Hue bridge authentication failed")]
    Authentication,

    #[error("Hue bridge returned HTTP status {0}")]
    HttpStatus(u16),

    #[error("unexpected Hue bridge response: {0}")]
    UnexpectedResponse(String),

    #[error("invalid Hue Entertainment configuration id: {0}")]
    InvalidEntertainmentConfigId(String),

    #[error("invalid hex secret: {0}")]
    InvalidHexSecret(String),

    #[error("Hue Entertainment DTLS failed: {0}")]
    Dtls(String),

    #[error("Hue Entertainment DTLS is not implemented yet")]
    DtlsNotImplemented,
}
