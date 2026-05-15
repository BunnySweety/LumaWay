//! Persist Hue bridge metadata into `lumaway.env`.

use anyhow::{Context, Result};
use lumaway_core::lumaway_main_env_path;
use lumaway_core::upsert_env_file;
use lumaway_hue::{bridge_tls_pinning_enabled, promote_bridge_tls_pin};
use tracing::info;

/// Writes `LUMAWAY_BRIDGE_ID` and promotes TLS pins to `hue-tls-pins/by-id/` when pinning is on.
pub fn persist_bridge_identity(bridge_ip: &str, bridge_id: &str) -> Result<()> {
    let bridge_ip = bridge_ip.trim();
    let bridge_id = bridge_id.trim();
    if bridge_ip.is_empty() || bridge_id.is_empty() {
        return Ok(());
    }

    upsert_env_file(
        &lumaway_main_env_path(),
        &[("LUMAWAY_BRIDGE_ID", bridge_id)],
    )
    .context("failed to update lumaway.env with LUMAWAY_BRIDGE_ID")?;

    if bridge_tls_pinning_enabled() {
        match promote_bridge_tls_pin(bridge_ip, bridge_id) {
            Ok(()) => info!(bridge_id, %bridge_ip, "TLS pin promoted to by-id path"),
            Err(error) => {
                tracing::warn!(
                    %error,
                    bridge_id,
                    %bridge_ip,
                    "could not promote TLS pin (connect once with pinning to create IP pin first)"
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumaway_core::read_env_file;
    use std::fs;

    #[test]
    fn persist_bridge_identity_writes_env_key() {
        let dir = std::env::temp_dir().join(format!("lumaway-bridge-env-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let old = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &dir);

        persist_bridge_identity("192.168.0.42", "001788fffeabc123").unwrap();

        let env_path = dir.join("lumaway").join("lumaway.env");
        let values = read_env_file(&env_path).unwrap();
        assert_eq!(
            values.get("LUMAWAY_BRIDGE_ID").map(String::as_str),
            Some("001788fffeabc123")
        );

        if let Some(old) = old {
            std::env::set_var("XDG_CONFIG_HOME", old);
        } else {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
