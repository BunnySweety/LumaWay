use std::io::{Read, Write};
use std::net::{IpAddr, Ipv6Addr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use openssl::ssl::{HandshakeError, SslConnector, SslMethod, SslVerifyMode};
use tracing::info;

use crate::{HueError, HueStreamMessage, Result};

/// PSK identity for DTLS on the entertainment channel.
///
/// Resolution order (aligned with Lumux):
/// 1. `LUMAWAY_DTLS_IDENTITY` — explicit string (trimmed, non-empty).
/// 2. `LUMAWAY_DTLS_USE_APP_KEY=1` — force the Hue **app key** (CLIP username) as identity.
/// 3. Otherwise: `hue-application-id` from `/auth/v1`; on failure, fall back to `app_key` with a warning.
///
/// Deprecated: `LUMAWAY_DTLS_USE_APPLICATION_ID` was the old opt-in for `/auth/v1`; the default now
/// tries that first, so this flag is ignored (kept so old env files do not break scripts).
pub async fn resolve_dtls_psk_identity(
    client: &crate::HueClient,
    app_key: &str,
) -> crate::Result<String> {
    if let Ok(value) = std::env::var("LUMAWAY_DTLS_IDENTITY") {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    if identity_env_flag("LUMAWAY_DTLS_USE_APP_KEY") {
        return Ok(app_key.to_string());
    }
    match client.application_id().await {
        Ok(id) => Ok(id),
        Err(err) => {
            tracing::warn!(
                error = %err,
                "hue-application-id unavailable; using app key as DTLS PSK identity (Lumux-compatible fallback)"
            );
            Ok(app_key.to_string())
        }
    }
}

fn identity_env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

const HUE_ENTERTAINMENT_PORT: u16 = 2100;
const HUE_PSK_CIPHERS: &str = "PSK-AES128-GCM-SHA256:PSK-CHACHA20-POLY1305";
const ENV_DTLS_ALLOW_REMOTE: &str = "LUMAWAY_DTLS_ALLOW_REMOTE";

/// Returns `true` when `LUMAWAY_DTLS_ALLOW_REMOTE=1` (or `true` / `yes` / `on`).
pub fn dtls_allows_non_lan_bridge_ip() -> bool {
    identity_env_flag(ENV_DTLS_ALLOW_REMOTE)
}

/// Hue entertainment UDP is intended for a bridge on the local LAN. By default, reject public or
/// loopback targets unless [`dtls_allows_non_lan_bridge_ip`].
pub fn validate_dtls_bridge_ip(bridge_ip: &str) -> Result<()> {
    let bridge_ip = bridge_ip.trim();
    if bridge_ip.is_empty() {
        return Err(HueError::Dtls("bridge IP is required for DTLS".into()));
    }
    let ip: IpAddr = bridge_ip
        .parse()
        .map_err(|_| HueError::Dtls(format!("invalid bridge IP for DTLS: {bridge_ip}")))?;
    if dtls_allows_non_lan_bridge_ip() || is_lan_dtls_target(ip) {
        return Ok(());
    }
    Err(HueError::Dtls(format!(
        "bridge IP {bridge_ip} is not a private/link-local address; set {ENV_DTLS_ALLOW_REMOTE}=1 to override (see docs/security.md)"
    )))
}

fn is_lan_dtls_target(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_private() && !v4.is_loopback(),
        IpAddr::V6(v6) => is_lan_ipv6(v6),
    }
}

fn is_lan_ipv6(ip: Ipv6Addr) -> bool {
    !ip.is_loopback() && (is_unique_local_ipv6(ip) || is_unicast_link_local_ipv6(ip))
}

fn is_unique_local_ipv6(ip: Ipv6Addr) -> bool {
    let octets = ip.octets();
    (octets[0] & 0xfe) == 0xfc
}

fn is_unicast_link_local_ipv6(ip: Ipv6Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 0xfe && (octets[1] & 0xc0) == 0x80
}

pub trait DtlsTransport {
    fn send(&mut self, message: &HueStreamMessage) -> Result<()>;
    fn send_bytes(&mut self, bytes: &[u8]) -> Result<()>;
    /// Read and discard DTLS records from the peer. OpenSSL may require this so writes keep working.
    fn drain_incoming(&mut self) -> Result<()> {
        Ok(())
    }
}

pub struct DtlsHueTransport {
    stream: openssl::ssl::SslStream<ConnectedUdpSocket>,
}

impl DtlsHueTransport {
    /// Connect to the bridge entertainment UDP port (`2100`).
    ///
    /// `psk_identity` is usually from [`resolve_dtls_psk_identity`] (`hue-application-id`, with optional
    /// `LUMAWAY_DTLS_IDENTITY` / `LUMAWAY_DTLS_USE_APP_KEY` overrides).
    ///
    /// # DTLS verification
    ///
    /// The session uses **PSK** ciphers only; OpenSSL is configured with **no peer certificate
    /// verification** because Hue does not present an X.509 chain comparable to HTTPS on this port.
    /// Confidentiality and authenticity of stream payloads rely on the **pre-shared key** and on UDP
    /// reaching the intended bridge IP. See `docs/security.md` in the repository for LAN threat
    /// considerations.
    pub fn connect(
        bridge_ip: &str,
        psk_identity: impl Into<String>,
        client_key_hex: impl AsRef<str>,
    ) -> Result<Self> {
        validate_dtls_bridge_ip(bridge_ip)?;
        let psk_identity = psk_identity.into();
        let psk = decode_hex(client_key_hex.as_ref())?;
        let (socket, remote) = ConnectedUdpSocket::connect(bridge_ip, HUE_ENTERTAINMENT_PORT)?;
        info!(
            bridge_ip,
            %remote,
            port = HUE_ENTERTAINMENT_PORT,
            psk_identity_len = psk_identity.len(),
            psk_len = psk.len(),
            "Hue entertainment DTLS UDP target resolved; starting handshake"
        );
        let connector = build_connector(psk_identity, psk)?;

        let mut cfg = connector
            .configure()
            .map_err(|e| HueError::Dtls(e.to_string()))?;
        cfg.set_verify_hostname(false);

        let mut ssl = cfg
            .into_ssl(bridge_ip)
            .map_err(|e| HueError::Dtls(e.to_string()))?;
        ssl.set_mtu(1200)
            .map_err(|e| HueError::Dtls(e.to_string()))?;
        let stream = ssl.connect(socket).map_err(map_handshake_error)?;
        info!(%remote, "Hue entertainment DTLS handshake completed");

        Ok(Self { stream })
    }
}

impl DtlsTransport for DtlsHueTransport {
    fn send(&mut self, message: &HueStreamMessage) -> Result<()> {
        self.send_bytes(message.as_bytes())
    }

    fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.stream
            .write_all(bytes)
            .map_err(|err| HueError::Dtls(err.to_string()))
    }

    fn drain_incoming(&mut self) -> Result<()> {
        drain_dtls_peer(&mut self.stream)
    }
}

struct ConnectedUdpSocket {
    inner: UdpSocket,
}

impl ConnectedUdpSocket {
    fn connect(host: &str, port: u16) -> Result<(Self, std::net::SocketAddr)> {
        let remote = (host, port)
            .to_socket_addrs()
            .map_err(|err| HueError::Dtls(err.to_string()))?
            .next()
            .ok_or_else(|| HueError::Dtls(format!("could not resolve {host}:{port}")))?;

        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|err| HueError::Dtls(err.to_string()))?;
        socket
            .connect(remote)
            .map_err(|err| HueError::Dtls(err.to_string()))?;
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|err| HueError::Dtls(err.to_string()))?;
        socket
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|err| HueError::Dtls(err.to_string()))?;

        Ok((Self { inner: socket }, remote))
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.inner.set_read_timeout(timeout)
    }
}

impl Read for ConnectedUdpSocket {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.recv(buf)
    }
}

impl Write for ConnectedUdpSocket {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.send(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// OpenSSL DTLS client for Hue entertainment: PSK-only, **no X.509 verification** (see
/// [`DtlsHueTransport::connect`]).
fn build_connector(psk_identity: String, psk: Vec<u8>) -> Result<SslConnector> {
    let mut builder = SslConnector::builder(SslMethod::dtls_client())
        .map_err(|err| HueError::Dtls(err.to_string()))?;

    builder.set_verify(SslVerifyMode::NONE);
    builder
        .set_cipher_list(HUE_PSK_CIPHERS)
        .map_err(|err| HueError::Dtls(err.to_string()))?;

    builder.set_psk_client_callback(move |_ssl, _hint, identity, psk_out| {
        write_psk_identity(&psk_identity, identity)?;
        if psk.len() > psk_out.len() {
            return Err(openssl::error::ErrorStack::get());
        }
        psk_out[..psk.len()].copy_from_slice(&psk);
        Ok(psk.len())
    });

    Ok(builder.build())
}

/// Read any DTLS packets the bridge sent (alerts, retransmits). If we only `write` and never read,
/// OpenSSL / the session can stall and the Hue bridge may stop entertainment after a few seconds.
fn drain_dtls_peer(stream: &mut openssl::ssl::SslStream<ConnectedUdpSocket>) -> Result<()> {
    const DRAIN_READ_TIMEOUT: Duration = Duration::from_millis(3);
    const NORMAL_READ_TIMEOUT: Duration = Duration::from_secs(5);
    const MAX_DRAIN_READS: usize = 64;

    stream
        .get_ref()
        .set_read_timeout(Some(DRAIN_READ_TIMEOUT))
        .map_err(|err| HueError::Dtls(err.to_string()))?;

    let mut scratch = [0u8; 2048];
    let drain_result = (|| {
        for _ in 0..MAX_DRAIN_READS {
            match stream.read(&mut scratch) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(err) => {
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) {
                        break;
                    }
                    let message = err.to_string();
                    if message.contains("WantRead")
                        || message.contains("WANT_READ")
                        || message.contains("want read")
                    {
                        break;
                    }
                    return Err(HueError::Dtls(err.to_string()));
                }
            }
        }
        Ok(())
    })();

    let _ = stream.get_ref().set_read_timeout(Some(NORMAL_READ_TIMEOUT));

    drain_result
}

fn map_handshake_error(error: HandshakeError<ConnectedUdpSocket>) -> HueError {
    match error {
        HandshakeError::SetupFailure(error) => HueError::Dtls(error.to_string()),
        HandshakeError::Failure(error) => HueError::Dtls(error.error().to_string()),
        HandshakeError::WouldBlock(_) => HueError::Dtls("DTLS handshake would block".into()),
    }
}

fn write_psk_identity(
    psk_identity: &str,
    identity_out: &mut [u8],
) -> std::result::Result<(), openssl::error::ErrorStack> {
    let identity = psk_identity.as_bytes();
    if identity.len() + 1 > identity_out.len() {
        return Err(openssl::error::ErrorStack::get());
    }

    identity_out[..identity.len()].copy_from_slice(identity);
    identity_out[identity.len()] = 0;
    Ok(())
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    let value = value.trim();
    if value.len() % 2 != 0 {
        return Err(HueError::InvalidHexSecret(
            "hex string must have an even length".into(),
        ));
    }

    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|err| HueError::InvalidHexSecret(err.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{decode_hex, validate_dtls_bridge_ip, ENV_DTLS_ALLOW_REMOTE};

    #[test]
    fn decodes_hex_client_key() {
        assert_eq!(decode_hex("0001020a0A").unwrap(), vec![0, 1, 2, 10, 10]);
    }

    #[test]
    fn rejects_odd_hex_secret_length() {
        assert!(decode_hex("abc").is_err());
    }

    #[test]
    fn rejects_invalid_hex_secret() {
        assert!(decode_hex("zz").is_err());
    }

    #[test]
    fn accepts_private_ipv4_for_dtls() {
        assert!(validate_dtls_bridge_ip("192.168.1.108").is_ok());
    }

    #[test]
    fn rejects_public_ipv4_for_dtls_by_default() {
        assert!(validate_dtls_bridge_ip("8.8.8.8").is_err());
    }

    #[test]
    fn allows_public_ipv4_when_env_set() {
        std::env::set_var(ENV_DTLS_ALLOW_REMOTE, "1");
        assert!(validate_dtls_bridge_ip("8.8.8.8").is_ok());
        std::env::remove_var(ENV_DTLS_ALLOW_REMOTE);
    }
}
