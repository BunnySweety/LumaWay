//! Profile files and `lumaway.env` loading helpers.

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROFILE_LOAD_ERROR_ENV: &str = "LUMAWAY_PROFILE_LOAD_ERROR";

pub fn load_profile_env_defaults() {
    let Some(profile_name) = std::env::var("LUMAWAY_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let path = match profile_path(&profile_name) {
        Ok(path) => path,
        Err(error) => {
            std::env::set_var(PROFILE_LOAD_ERROR_ENV, error.to_string());
            return;
        }
    };
    if !path.exists() {
        std::env::set_var(
            PROFILE_LOAD_ERROR_ENV,
            format!(
                "LUMAWAY_PROFILE={} points to missing profile file: {}",
                profile_name,
                path.display()
            ),
        );
        return;
    }

    match read_key_value_file(&path) {
        Ok(values) => {
            for (key, value) in values {
                if is_profile_key(&key) && std::env::var_os(&key).is_none() {
                    std::env::set_var(key, value);
                }
            }
        }
        Err(error) => std::env::set_var(PROFILE_LOAD_ERROR_ENV, error.to_string()),
    }
}

pub fn load_main_env_defaults() {
    let path = main_env_path();
    let Ok(values) = read_key_value_file(&path) else {
        return;
    };
    for (key, value) in values {
        if is_main_env_key(&key) {
            std::env::set_var(key, value);
        }
    }
}

pub fn main_env_path() -> PathBuf {
    config_home().join("lumaway").join("lumaway.env")
}

pub fn ensure_profile_loaded() -> Result<()> {
    if let Ok(error) = std::env::var(PROFILE_LOAD_ERROR_ENV) {
        bail!("{error}");
    }
    Ok(())
}

pub fn write_profile_template(name: &str, force: bool) -> Result<()> {
    let path = profile_path(name)?;
    write_profile_file(&path, default_profile_text("auto", 120, 68), force)?;
    println!("profile_template path={}", path.display());
    Ok(())
}

pub fn list_profiles() -> Result<()> {
    for (name, path) in available_profiles()? {
        println!("profile name={name} path={}", path.display());
    }
    Ok(())
}

pub fn available_profiles() -> Result<Vec<(String, PathBuf)>> {
    let dir = config_home().join("lumaway").join("profiles");
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("env") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
            if profile_path(stem).is_ok() {
                profiles.push((stem.to_string(), path));
            }
        }
    }
    profiles.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(profiles)
}

pub fn write_profile_file(path: &Path, text: String, force: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() && !force {
        bail!(
            "profile already exists: {} (use --force to overwrite)",
            path.display()
        );
    }
    fs::write(path, text)?;
    Ok(())
}

pub fn default_profile_text(
    capture_backend: &str,
    sample_width: i32,
    sample_height: i32,
) -> String {
    format!(
        "LUMAWAY_SYNC_MODE=video\nLUMAWAY_CAPTURE_BACKEND={capture_backend}\nLUMAWAY_CAPTURE_FPS=8\nLUMAWAY_STREAM_FPS=25\nLUMAWAY_PIPEWIRE_FPS=25\nLUMAWAY_CAPTURE_POLL_MS=5\nLUMAWAY_SAMPLE_WIDTH={sample_width}\nLUMAWAY_SAMPLE_HEIGHT={sample_height}\nLUMAWAY_SAMPLE_EDGE_MARGIN=0.08\nLUMAWAY_SAMPLE_CROP_LEFT=0.0000\nLUMAWAY_SAMPLE_CROP_RIGHT=0.0000\nLUMAWAY_SAMPLE_CROP_TOP=0.0000\nLUMAWAY_SAMPLE_CROP_BOTTOM=0.0000\nLUMAWAY_SAMPLING=region\nLUMAWAY_BRIGHTNESS=1.00\nLUMAWAY_REACTIVITY=0.35\nLUMAWAY_COLOR_PROFILE=vivid\nLUMAWAY_NOISE_THRESHOLD=3\n"
    )
}

pub fn profile_path(name: &str) -> Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        bail!("invalid profile name: {name}");
    }
    Ok(config_home()
        .join("lumaway")
        .join("profiles")
        .join(format!("{name}.env")))
}

pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn read_key_value_file(path: &Path) -> Result<HashMap<String, String>> {
    let text = fs::read_to_string(path)?;
    let mut values = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok(values)
}

pub fn is_profile_key(key: &str) -> bool {
    matches!(
        key,
        "LUMAWAY_SYNC_MODE"
            | "LUMAWAY_PRESET"
            | "LUMAWAY_CAPTURE_BACKEND"
            | "LUMAWAY_CAPTURE_FPS"
            | "LUMAWAY_STREAM_FPS"
            | "LUMAWAY_PIPEWIRE_FPS"
            | "LUMAWAY_CAPTURE_POLL_MS"
            | "LUMAWAY_SAMPLE_WIDTH"
            | "LUMAWAY_SAMPLE_HEIGHT"
            | "LUMAWAY_SAMPLE_EDGE_MARGIN"
            | "LUMAWAY_SAMPLE_CROP_LEFT"
            | "LUMAWAY_SAMPLE_CROP_RIGHT"
            | "LUMAWAY_SAMPLE_CROP_TOP"
            | "LUMAWAY_SAMPLE_CROP_BOTTOM"
            | "LUMAWAY_SAMPLING"
            | "LUMAWAY_BRIGHTNESS"
            | "LUMAWAY_REACTIVITY"
            | "LUMAWAY_COLOR_PROFILE"
            | "LUMAWAY_NOISE_THRESHOLD"
            | "LUMAWAY_MAX_STEP"
    )
}

pub fn is_main_env_key(key: &str) -> bool {
    key.starts_with("LUMAWAY_") || key == "RUST_LOG"
}
