//! `lumaway doctor` — environment and dependency checks.

use anyhow::Result;
use std::process::Command;
use tracing::warn;

use lumaway_hue::{
    bridge_id_from_env, bridge_pin_file_path, bridge_pin_paths, bridge_tls_pin_kind,
    bridge_tls_pinning_enabled, validate_dtls_bridge_ip, BridgeTlsPinKind, HueBridgeConfig,
    HueClient, HueError,
};

use crate::bridge_env::persist_bridge_identity;

use crate::profile_env::{
    config_home as xdg_config_home, home_dir, is_profile_key, main_env_path, profile_path,
    read_key_value_file, PROFILE_LOAD_ERROR_ENV,
};

#[derive(Debug, Clone, Copy)]
enum CheckStatus {
    Ok,
    Warning,
    Error,
}

impl CheckStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

fn print_check(status: CheckStatus, id: &str, message: impl AsRef<str>) {
    println!("[{}] {id}: {}", status.label(), message.as_ref());
}

pub async fn run_doctor(bridge: Option<String>, app_key: Option<String>) -> Result<()> {
    let bridge = bridge.or_else(|| non_empty_env("LUMAWAY_BRIDGE"));
    let app_key = app_key.or_else(|| non_empty_env("LUMAWAY_APP_KEY"));
    let client_key = non_empty_env("LUMAWAY_CLIENT_KEY");

    print_check(
        CheckStatus::Ok,
        "platform.target",
        "Linux Wayland focused build",
    );
    check_config_paths();

    check_command("pkg-config", "pkg-config");
    check_pkg_config(
        "gstreamer.dev",
        &["gstreamer-1.0", "gstreamer-app-1.0", "gstreamer-video-1.0"],
    );
    check_gstreamer_element("gstreamer.pipewiresrc", "pipewiresrc", true);
    check_gstreamer_element("gstreamer.videoconvert", "videoconvert", true);
    check_gstreamer_element("gstreamer.appsink", "appsink", true);
    check_gstreamer_element("gstreamer.glupload", "glupload", false);
    check_gstreamer_element("gstreamer.glcolorconvert", "glcolorconvert", false);
    check_gstreamer_element("gstreamer.gldownload", "gldownload", false);
    check_portal_environment();
    check_profile_configuration();
    check_user_command(
        "portal.dbus",
        "busctl",
        &["--user", "status", "org.freedesktop.portal.Desktop"],
    );
    check_user_command(
        "portal.service",
        "systemctl",
        &["--user", "is-active", "xdg-desktop-portal.service"],
    );
    check_user_command(
        "portal.gnome",
        "systemctl",
        &["--user", "is-active", "xdg-desktop-portal-gnome.service"],
    );
    check_command("openssl", "openssl");

    check_hue_bridge_tls_pinning(bridge.as_deref());
    check_hue_dtls_readiness(bridge.as_deref(), client_key.as_deref());

    match (bridge, app_key) {
        (Some(bridge), Some(app_key)) => {
            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge.clone(),
                app_key: Some(app_key),
                client_key: None,
            })?;

            match client.entertainment_areas().await {
                Ok(areas) => print_check(
                    CheckStatus::Ok,
                    "hue.entertainment_areas",
                    format!("{} area(s) found on {bridge}", areas.len()),
                ),
                Err(error) => print_hue_error_check("hue.entertainment_areas", &error),
            }

            match client.application_id().await {
                Ok(_) => print_check(
                    CheckStatus::Ok,
                    "hue.application_id",
                    "hue-application-id header is available",
                ),
                Err(error) => print_hue_error_check("hue.application_id", &error),
            }

            if bridge_tls_pinning_enabled() {
                match client.bridge_info().await {
                    Ok(info) => {
                        print_check(
                            CheckStatus::Ok,
                            "hue.bridge_info",
                            format!("bridge id {} ({})", info.id, info.name),
                        );
                        match persist_bridge_identity(&bridge, &info.id) {
                            Ok(()) => print_check(
                                CheckStatus::Ok,
                                "hue.bridge_id",
                                format!("LUMAWAY_BRIDGE_ID saved to lumaway.env ({})", info.id),
                            ),
                            Err(error) => print_check(
                                CheckStatus::Warning,
                                "hue.bridge_id",
                                format!("could not save LUMAWAY_BRIDGE_ID: {error}"),
                            ),
                        }
                    }
                    Err(error) => print_hue_error_check("hue.bridge_info", &error),
                }
            }
        }
        (Some(_), None) => print_check(
            CheckStatus::Warning,
            "hue.credentials",
            "bridge was provided but no app key was provided",
        ),
        (None, _) => print_check(
            CheckStatus::Warning,
            "hue.bridge",
            "no bridge provided; skipping Hue checks",
        ),
    }

    Ok(())
}

fn check_hue_dtls_readiness(bridge_ip: Option<&str>, client_key: Option<&str>) {
    match client_key {
        Some(key) if key.len() >= 32 => print_check(
            CheckStatus::Ok,
            "hue.dtls.client_key",
            "LUMAWAY_CLIENT_KEY is set (hex streaming secret)",
        ),
        Some(_) => print_check(
            CheckStatus::Warning,
            "hue.dtls.client_key",
            "LUMAWAY_CLIENT_KEY looks short; pairing may be incomplete",
        ),
        None => print_check(
            CheckStatus::Warning,
            "hue.dtls.client_key",
            "LUMAWAY_CLIENT_KEY unset; sync and test-color need it for DTLS",
        ),
    }

    let Some(ip) = bridge_ip else {
        print_check(
            CheckStatus::Ok,
            "hue.dtls.bridge_ip",
            "no bridge IP; skipping DTLS target check",
        );
        return;
    };

    match validate_dtls_bridge_ip(ip) {
        Ok(()) => print_check(
            CheckStatus::Ok,
            "hue.dtls.bridge_ip",
            format!("{ip} is acceptable for entertainment UDP (private/link-local)"),
        ),
        Err(HueError::Dtls(message)) => {
            print_check(CheckStatus::Warning, "hue.dtls.bridge_ip", message)
        }
        Err(error) => print_check(
            CheckStatus::Warning,
            "hue.dtls.bridge_ip",
            error.to_string(),
        ),
    }
}

fn check_hue_bridge_tls_pinning(bridge_ip: Option<&str>) {
    let raw = std::env::var("LUMAWAY_HUE_PIN_CERTS").unwrap_or_default();
    let raw_display = if raw.trim().is_empty() {
        "unset".to_string()
    } else {
        raw
    };
    if bridge_tls_pinning_enabled() {
        let kind = match bridge_tls_pin_kind() {
            BridgeTlsPinKind::Spki => "spki (default)",
            BridgeTlsPinKind::LeafCertDer => "leaf-cert DER (legacy)",
        };
        print_check(
            CheckStatus::Ok,
            "hue.tls_pinning",
            format!("LUMAWAY_HUE_PIN_CERTS enabled ({raw_display}); pin mode: {kind}"),
        );
    } else {
        print_check(
            CheckStatus::Ok,
            "hue.tls_pinning",
            format!("LUMAWAY_HUE_PIN_CERTS disabled ({raw_display}); relaxed TLS for Hue bridge"),
        );
    }

    if let Ok(dir) = std::env::var("LUMAWAY_HUE_PIN_DIR") {
        if !dir.trim().is_empty() {
            print_check(
                CheckStatus::Ok,
                "hue.tls_pin.config_root",
                format!("LUMAWAY_HUE_PIN_DIR overrides Lumaway config root: {dir}"),
            );
        }
    }

    if let Some(id) = bridge_id_from_env() {
        print_check(
            CheckStatus::Ok,
            "hue.tls_pin.bridge_id",
            format!("LUMAWAY_BRIDGE_ID set: {id}"),
        );
    }

    let Some(ip) = bridge_ip else {
        print_check(
            CheckStatus::Ok,
            "hue.tls_pin.file",
            "no bridge IP; pin file path depends on LUMAWAY_BRIDGE / --bridge",
        );
        return;
    };

    if !bridge_tls_pinning_enabled() {
        print_check(
            CheckStatus::Ok,
            "hue.tls_pin.file",
            "pinning disabled; no pin file is used",
        );
        return;
    }

    let paths = bridge_pin_paths(ip, bridge_id_from_env().as_deref(), bridge_tls_pin_kind());
    if let Some(found) = paths.iter().find(|p| p.exists()) {
        print_check(
            CheckStatus::Ok,
            "hue.tls_pin.file",
            format!("pin file present: {}", found.display()),
        );
    } else {
        let primary = bridge_pin_file_path(ip);
        print_check(
            CheckStatus::Warning,
            "hue.tls_pin.file",
            format!(
                "no pin file yet (created on first successful HTTPS); expected e.g. {}",
                primary.display()
            ),
        );
    }
}

fn print_hue_error_check(id: &str, error: &HueError) {
    match error {
        HueError::Authentication => print_check(
            CheckStatus::Error,
            id,
            "saved Hue application key was rejected; open Settings, press Pair on the bridge, then press Pair in LumaWay to refresh LUMAWAY_APP_KEY and LUMAWAY_CLIENT_KEY",
        ),
        HueError::MissingAppKey => print_check(
            CheckStatus::Warning,
            id,
            "no Hue application key is configured; use Settings > Pair",
        ),
        HueError::TlsPin(message) => print_check(
            CheckStatus::Error,
            id,
            format!("Hue TLS certificate pinning: {message}"),
        ),
        HueError::Request(message) => print_check(
            CheckStatus::Error,
            id,
            format!("Hue bridge request failed: {message}"),
        ),
        _ => print_check(CheckStatus::Error, id, error.to_string()),
    }
}

fn check_config_paths() {
    let home = home_dir();
    let config_dir = xdg_config_home();
    let env_path = main_env_path();
    print_check(
        CheckStatus::Ok,
        "config.home",
        format!("HOME={}", home.display()),
    );
    print_check(
        CheckStatus::Ok,
        "config.dir",
        config_dir.display().to_string(),
    );
    if env_path.exists() {
        print_check(
            CheckStatus::Ok,
            "config.file",
            format!("using {}", env_path.display()),
        );
    } else {
        print_check(
            CheckStatus::Warning,
            "config.file",
            format!(
                "missing {}; GUI-saved settings will not be loaded",
                env_path.display()
            ),
        );
    }
}

pub fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn check_profile_configuration() {
    if let Ok(error) = std::env::var(PROFILE_LOAD_ERROR_ENV) {
        print_check(CheckStatus::Error, "profile.load", error);
        return;
    }

    let Some(profile_name) = std::env::var("LUMAWAY_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        print_check(
            CheckStatus::Warning,
            "profile.selected",
            "LUMAWAY_PROFILE is not set; using CLI/env defaults",
        );
        return;
    };

    let path = match profile_path(&profile_name) {
        Ok(path) => path,
        Err(error) => {
            print_check(CheckStatus::Error, "profile.name", error.to_string());
            return;
        }
    };
    if !path.exists() {
        print_check(
            CheckStatus::Error,
            "profile.file",
            format!("profile file does not exist: {}", path.display()),
        );
        return;
    }

    match read_key_value_file(&path) {
        Ok(values) => {
            let ignored: Vec<_> = values
                .keys()
                .filter(|key| !is_profile_key(key))
                .cloned()
                .collect();
            if ignored.is_empty() {
                print_check(
                    CheckStatus::Ok,
                    "profile.file",
                    format!(
                        "{} loaded with {} supported setting(s)",
                        path.display(),
                        values.len()
                    ),
                );
            } else {
                print_check(
                    CheckStatus::Warning,
                    "profile.ignored_keys",
                    format!("ignored unsupported key(s): {}", ignored.join(", ")),
                );
            }
        }
        Err(error) => print_check(CheckStatus::Error, "profile.file", error.to_string()),
    }
}

fn check_portal_environment() {
    for key in ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS"] {
        if std::env::var_os(key).is_some() {
            print_check(CheckStatus::Ok, &format!("env.{key}"), "set");
        } else {
            print_check(
                CheckStatus::Warning,
                &format!("env.{key}"),
                "missing; Portal calls must run inside the graphical user session",
            );
        }
    }

    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        print_check(CheckStatus::Ok, "env.WAYLAND_DISPLAY", "set");
    } else if std::env::var_os("DISPLAY").is_some() {
        print_check(
            CheckStatus::Warning,
            "env.WAYLAND_DISPLAY",
            "missing, but DISPLAY is set; LumaWay is intended for Wayland Portal capture",
        );
    } else {
        print_check(
            CheckStatus::Warning,
            "env.WAYLAND_DISPLAY",
            "missing; no graphical Wayland session detected",
        );
    }
}

fn check_command(id: &str, command: &str) {
    match Command::new(command).arg("--version").output() {
        Ok(output) if output.status.success() => {
            print_check(CheckStatus::Ok, id, format!("{command} is available"));
        }
        Ok(output) => {
            warn!(?output.status, command, "doctor command check failed");
            print_check(
                CheckStatus::Warning,
                id,
                format!("{command} is present but returned a non-zero status"),
            );
        }
        Err(error) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!("{command} not found: {error}"),
            );
        }
    }
}

fn check_user_command(id: &str, command: &str, args: &[&str]) {
    match Command::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = stdout.lines().next().unwrap_or("available");
            print_check(CheckStatus::Ok, id, detail);
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail = stderr.lines().next().unwrap_or("command returned non-zero");
            print_check(CheckStatus::Warning, id, detail);
        }
        Err(error) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!("{command} failed: {error}"),
            );
        }
    }
}

fn check_gstreamer_element(id: &str, element: &str, required: bool) {
    match Command::new("gst-inspect-1.0").arg(element).output() {
        Ok(output) if output.status.success() => {
            print_check(CheckStatus::Ok, id, format!("{element} is available"));
        }
        Ok(_) if required => {
            print_check(
                CheckStatus::Error,
                id,
                format!("{element} is missing; capture cannot run without it"),
            );
        }
        Ok(_) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!("{element} is missing; optional acceleration path unavailable"),
            );
        }
        Err(error) if required => {
            print_check(
                CheckStatus::Error,
                id,
                format!("gst-inspect-1.0 failed for {element}: {error}"),
            );
        }
        Err(error) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!("gst-inspect-1.0 failed for {element}: {error}"),
            );
        }
    }
}

fn check_pkg_config(id: &str, packages: &[&str]) {
    match Command::new("pkg-config")
        .arg("--exists")
        .args(packages)
        .status()
    {
        Ok(status) if status.success() => {
            print_check(
                CheckStatus::Ok,
                id,
                format!("found {}", packages.join(", ")),
            );
        }
        Ok(_) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!(
                    "missing {}; install development packages before the capture spike",
                    packages.join(", ")
                ),
            );
        }
        Err(error) => {
            print_check(
                CheckStatus::Warning,
                id,
                format!("pkg-config failed: {error}"),
            );
        }
    }
}
