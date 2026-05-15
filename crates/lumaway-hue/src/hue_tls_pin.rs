//! Optional TLS certificate pinning for HTTPS to the Hue bridge.
//!
//! Default pin: SHA-256 of the leaf **SubjectPublicKeyInfo** (SPKI). Legacy mode pins the full leaf DER.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, WebPkiSupportedAlgorithms};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::DigitallySignedStruct;
use rustls::{ClientConfig, Error, SignatureScheme};
use sha2::{Digest, Sha256};
use tracing::info;
use webpki::EndEntityCert;

use crate::HueError;

const ENV_PIN: &str = "LUMAWAY_HUE_PIN_CERTS";
const ENV_PIN_DIR: &str = "LUMAWAY_HUE_PIN_DIR";
const ENV_PIN_MODE: &str = "LUMAWAY_HUE_PIN_MODE";
const ENV_BRIDGE_ID: &str = "LUMAWAY_BRIDGE_ID";

/// What material is hashed into the 32-byte pin file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeTlsPinKind {
    /// SHA-256 of RFC 5280 `SubjectPublicKeyInfo` (default; survives leaf re-issue with same key).
    Spki,
    /// SHA-256 of the leaf certificate DER (legacy).
    LeafCertDer,
}

/// `true` when `LUMAWAY_HUE_PIN_CERTS` is set to a truthy value (`1`, `true`, `yes`, `on`).
pub fn bridge_tls_pinning_enabled() -> bool {
    truthy_env(std::env::var(ENV_PIN).ok().as_deref())
}

/// Active pin kind (`spki` default; `cert` / `der` / `leaf` select legacy leaf-DER hashing).
pub fn bridge_tls_pin_kind() -> BridgeTlsPinKind {
    match std::env::var(ENV_PIN_MODE)
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("cert") | Some("der") | Some("leaf") => BridgeTlsPinKind::LeafCertDer,
        _ => BridgeTlsPinKind::Spki,
    }
}

/// Optional bridge id from `LUMAWAY_BRIDGE_ID` (used for pin lookup before REST returns bridge info).
pub fn bridge_id_from_env() -> Option<String> {
    non_empty_env(ENV_BRIDGE_ID)
}

/// Primary pin path for `doctor` and operators (SPKI or legacy name for the configured mode).
pub fn bridge_pin_file_path(bridge_ip: &str) -> PathBuf {
    bridge_pin_paths(
        bridge_ip,
        bridge_id_from_env().as_deref(),
        bridge_tls_pin_kind(),
    )
    .into_iter()
    .next()
    .expect("bridge_pin_paths is never empty")
}

/// Pin paths to try, most specific first (`by-id` then IP; legacy `.sha256` last when using SPKI).
pub fn bridge_pin_paths(
    bridge_ip: &str,
    bridge_id: Option<&str>,
    kind: BridgeTlsPinKind,
) -> Vec<PathBuf> {
    let root = lumaway_config_root().join("hue-tls-pins");
    let mut paths = Vec::new();

    if let Some(id) = bridge_id.filter(|s| !s.trim().is_empty()) {
        paths.push(root.join("by-id").join(pin_filename(sanitize_id(id), kind)));
    }

    paths.push(root.join(pin_filename(sanitize_ip(bridge_ip), kind)));

    if kind == BridgeTlsPinKind::Spki {
        paths.push(root.join(format!("{}.sha256", sanitize_ip(bridge_ip))));
    }

    paths
}

/// After the bridge id is known (e.g. from `bridge_info`), copy an existing IP pin to `by-id/`.
pub fn promote_bridge_tls_pin(bridge_ip: &str, bridge_id: &str) -> Result<(), HueError> {
    if !bridge_tls_pinning_enabled() {
        return Ok(());
    }
    let id = bridge_id.trim();
    if id.is_empty() {
        return Ok(());
    }

    let kind = bridge_tls_pin_kind();
    let sources = bridge_pin_paths(bridge_ip, None, kind);
    let source = sources.iter().find(|p| p.exists()).ok_or_else(|| {
        HueError::TlsPin("no pin file to promote (connect once with pinning enabled)".into())
    })?;

    let dest = lumaway_config_root()
        .join("hue-tls-pins")
        .join("by-id")
        .join(pin_filename(sanitize_id(id), kind));

    if source == &dest {
        return Ok(());
    }

    if dest.exists() {
        let existing = fs::read(&dest)
            .map_err(|e| HueError::TlsPin(format!("read {}: {e}", dest.display())))?;
        let current = fs::read(source)
            .map_err(|e| HueError::TlsPin(format!("read {}: {e}", source.display())))?;
        if existing == current {
            return Ok(());
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| HueError::TlsPin(format!("mkdir {}: {e}", parent.display())))?;
    }
    fs::copy(source, &dest)
        .map_err(|e| HueError::TlsPin(format!("copy pin to {}: {e}", dest.display())))?;
    restrict_pin_file_permissions(&dest)
        .map_err(|e| HueError::TlsPin(format!("chmod {}: {e}", dest.display())))?;
    info!(
        from = %source.display(),
        to = %dest.display(),
        "promoted Hue bridge TLS pin to bridge-id path"
    );
    Ok(())
}

fn pin_filename(safe_key: String, kind: BridgeTlsPinKind) -> String {
    match kind {
        BridgeTlsPinKind::Spki => format!("{safe_key}.spki.sha256"),
        BridgeTlsPinKind::LeafCertDer => format!("{safe_key}.sha256"),
    }
}

fn sanitize_ip(bridge_ip: &str) -> String {
    bridge_ip
        .trim()
        .replace(':', "-")
        .replace('/', "_")
        .replace('\\', "_")
}

fn sanitize_id(bridge_id: &str) -> String {
    bridge_id
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn pin_kind_for_path(path: &Path) -> BridgeTlsPinKind {
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.ends_with(".spki.sha256"))
    {
        BridgeTlsPinKind::Spki
    } else {
        BridgeTlsPinKind::LeafCertDer
    }
}

fn compute_pin_digest(der: &[u8], kind: BridgeTlsPinKind) -> Result<[u8; 32], Error> {
    let digest = match kind {
        BridgeTlsPinKind::LeafCertDer => Sha256::digest(der),
        BridgeTlsPinKind::Spki => {
            let cert = CertificateDer::from(der.to_vec());
            let parsed = EndEntityCert::try_from(&cert)
                .map_err(|_| Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?;
            Sha256::digest(parsed.subject_public_key_info().as_ref())
        }
    };
    let mut out = [0_u8; 32];
    out.copy_from_slice(&digest);
    Ok(out)
}

fn truthy_env(value: Option<&str>) -> bool {
    match value.map(str::trim) {
        None | Some("") => false,
        Some(s) => {
            let lower = s.to_ascii_lowercase();
            matches!(lower.as_str(), "1" | "true" | "yes" | "on")
        }
    }
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn lumaway_config_root() -> PathBuf {
    if let Ok(dir) = std::env::var(ENV_PIN_DIR) {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .map(|p| p.join("lumaway"))
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_default();
            PathBuf::from(home).join(".config/lumaway")
        })
}

#[derive(Debug)]
struct LoadedPin {
    digest: [u8; 32],
    kind: BridgeTlsPinKind,
}

fn load_pin(paths: &[PathBuf]) -> Result<Option<LoadedPin>, HueError> {
    for path in paths {
        if !path.exists() {
            continue;
        }
        let bytes = fs::read(path)
            .map_err(|e| HueError::TlsPin(format!("read {}: {e}", path.display())))?;
        if bytes.len() != 32 {
            return Err(HueError::TlsPin(format!(
                "pin file {}: expected 32 bytes, got {}",
                path.display(),
                bytes.len()
            )));
        }
        let mut digest = [0_u8; 32];
        digest.copy_from_slice(&bytes);
        return Ok(Some(LoadedPin {
            digest,
            kind: pin_kind_for_path(path),
        }));
    }
    Ok(None)
}

#[derive(Debug)]
struct HueBridgeTlsVerifier {
    algorithms: WebPkiSupportedAlgorithms,
    write_path: PathBuf,
    expected: Option<LoadedPin>,
    learn_kind: BridgeTlsPinKind,
}

impl ServerCertVerifier for HueBridgeTlsVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let der = end_entity.as_ref();
        if der.is_empty() {
            return Err(Error::InvalidCertificate(
                rustls::CertificateError::BadEncoding,
            ));
        }

        match self.expected {
            Some(ref loaded) => {
                let observed = compute_pin_digest(der, loaded.kind)?;
                if !constant_time_eq_32(&loaded.digest, &observed) {
                    return Err(Error::General(
                        "Hue bridge TLS pin mismatch (see docs/security.md)".into(),
                    ));
                }
            }
            None => {
                let hash = compute_pin_digest(der, self.learn_kind)?;
                persist_new_pin(&self.write_path, &hash).map_err(|e| {
                    Error::General(format!(
                        "Hue TLS pin: could not write {}: {e}",
                        self.write_path.display()
                    ))
                })?;
            }
        }

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

fn constant_time_eq_32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter()
        .zip(b.iter())
        .fold(0_u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn persist_new_pin(path: &Path, hash: &[u8; 32]) -> io::Result<()> {
    if path.exists() {
        let existing = fs::read(path)?;
        if existing.len() == 32 {
            let mut prev = [0_u8; 32];
            prev.copy_from_slice(&existing);
            if constant_time_eq_32(&prev, hash) {
                let _ = restrict_pin_file_permissions(path);
                return Ok(());
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pin file exists with a different fingerprint",
        ));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, hash)?;
    fs::rename(&tmp, path)?;
    restrict_pin_file_permissions(path)?;
    info!(
        path = %path.display(),
        "stored new Hue bridge TLS pin"
    );
    Ok(())
}

#[cfg(unix)]
fn restrict_pin_file_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perm = fs::metadata(path)?.permissions();
    perm.set_mode(0o600);
    fs::set_permissions(path, perm)
}

#[cfg(not(unix))]
fn restrict_pin_file_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

pub(crate) fn build_hue_http_client(bridge_ip: &str) -> Result<reqwest::Client, HueError> {
    let mut builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5));

    if bridge_tls_pinning_enabled() {
        let kind = bridge_tls_pin_kind();
        let bridge_id = bridge_id_from_env();
        let paths = bridge_pin_paths(bridge_ip, bridge_id.as_deref(), kind);
        let write_path = paths
            .iter()
            .find(|p| pin_kind_for_path(p) == kind)
            .cloned()
            .expect("bridge_pin_paths includes mode-specific path");
        let expected = load_pin(&paths)?;
        let algorithms = rustls::crypto::ring::default_provider().signature_verification_algorithms;
        let verifier = Arc::new(HueBridgeTlsVerifier {
            algorithms,
            write_path,
            expected,
            learn_kind: kind,
        });
        let tls = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(verifier)
            .with_no_client_auth();

        builder = builder.use_preconfigured_tls(tls);
    } else {
        builder = builder.danger_accept_invalid_certs(true);
    }

    builder
        .build()
        .map_err(|e| HueError::Request(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_env_values() {
        assert!(!truthy_env(None));
        assert!(!truthy_env(Some("")));
        assert!(truthy_env(Some("1")));
    }

    #[test]
    fn pin_kind_from_env_defaults_to_spki() {
        std::env::remove_var(ENV_PIN_MODE);
        assert_eq!(bridge_tls_pin_kind(), BridgeTlsPinKind::Spki);
    }

    #[test]
    fn bridge_pin_path_sanitizes_ip_and_id() {
        let _guard = temp_env();
        let p = bridge_pin_file_path("192.168.0.1");
        assert!(p.to_string_lossy().ends_with("192.168.0.1.spki.sha256"));
        let paths = bridge_pin_paths(
            "192.168.0.1",
            Some("001788:fffe:123456"),
            BridgeTlsPinKind::Spki,
        );
        assert!(paths[0].to_string_lossy().contains("by-id"));
        assert!(paths[0].to_string_lossy().contains("001788_fffe_123456"));
    }

    #[test]
    fn constant_time_eq() {
        let a = [1_u8; 32];
        let mut b = [1_u8; 32];
        assert!(constant_time_eq_32(&a, &b));
        b[31] = 2;
        assert!(!constant_time_eq_32(&a, &b));
    }

    fn temp_env() -> impl Drop {
        struct Guard;
        impl Drop for Guard {
            fn drop(&mut self) {
                std::env::remove_var(ENV_PIN_DIR);
                std::env::remove_var(ENV_PIN_MODE);
                std::env::remove_var(ENV_BRIDGE_ID);
            }
        }
        let dir = std::env::temp_dir().join(format!("lumaway-hue-pin-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        std::env::set_var(ENV_PIN_DIR, dir.as_os_str());
        Guard
    }

    #[test]
    fn load_pin_rejects_wrong_length() {
        let _g = temp_env();
        let path = bridge_pin_file_path("10.0.0.5");
        let _ = fs::remove_file(&path);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, b"short").unwrap();
        assert!(load_pin(&[path.clone()]).is_err());
        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn pin_restricts_file_mode_on_unix() {
        let dir = std::env::temp_dir().join(format!("lumaway-hue-pinmode-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.sha256");
        fs::write(&path, [2u8; 32]).unwrap();
        super::restrict_pin_file_permissions(&path).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }
}
