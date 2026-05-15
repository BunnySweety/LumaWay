pub mod client;
pub mod color;
pub mod dtls;
pub mod entertainment;
pub mod error;
mod hue_tls_pin;

pub use client::{
    discover_bridges, BridgeDiscovery, BridgeInfo, BridgeUser, EntertainmentArea,
    EntertainmentChannel, EntertainmentChannelPosition, HueClient,
};
pub use color::HueColor;
pub use dtls::{
    dtls_allows_non_lan_bridge_ip, resolve_dtls_psk_identity, validate_dtls_bridge_ip,
    DtlsHueTransport, DtlsTransport,
};
pub use entertainment::{ChannelColor, HueStreamEncoder, HueStreamFrame, HueStreamMessage};
pub use error::{HueError, Result};
pub use hue_tls_pin::{
    bridge_id_from_env, bridge_pin_file_path, bridge_pin_paths, bridge_tls_pin_kind,
    bridge_tls_pinning_enabled, promote_bridge_tls_pin, BridgeTlsPinKind,
};

#[derive(Debug, Clone)]
pub struct HueBridgeConfig {
    pub bridge_ip: String,
    pub app_key: Option<String>,
    pub client_key: Option<String>,
}

impl HueBridgeConfig {
    pub fn new(bridge_ip: impl Into<String>) -> Self {
        Self {
            bridge_ip: bridge_ip.into(),
            app_key: None,
            client_key: None,
        }
    }
}
