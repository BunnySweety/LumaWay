//! XDG config `key=value` env files (e.g. `~/.config/lumaway/lumaway.env`).

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{config_v1_updates, CoreError, Result};

pub fn config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn lumaway_main_env_path() -> PathBuf {
    config_home().join("lumaway").join("lumaway.env")
}

pub fn migrate_lumaway_env_v1(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let values = read_env_file(path)?;
    let updates = config_v1_updates(&values);
    let update_refs: Vec<(&str, &str)> = updates
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    upsert_env_file(path, &update_refs)
}

/// Merge `updates` into an env file (create or patch lines). Preserves comments and unknown keys.
pub fn upsert_env_file(path: &Path, updates: &[(&str, &str)]) -> Result<()> {
    if updates.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| CoreError::Io {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    let mut lines: Vec<String> = if path.exists() {
        fs::read_to_string(path)
            .map_err(|e| io_error(path, e))?
            .lines()
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };

    for (key, value) in updates {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let mut found = false;
        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((existing_key, _)) = trimmed.split_once('=') {
                if existing_key.trim() == key {
                    *line = format!("{key}={value}");
                    found = true;
                    break;
                }
            }
        }
        if !found {
            lines.push(format!("{key}={value}"));
        }
    }

    let text = if lines.is_empty() {
        String::new()
    } else {
        let mut text = lines.join("\n");
        text.push('\n');
        text
    };

    let tmp = path.with_extension("env.tmp");
    fs::write(&tmp, text).map_err(|e| io_error(&tmp, e))?;
    fs::rename(&tmp, path).map_err(|e| io_error(path, e))?;
    restrict_env_file_permissions(path)?;
    Ok(())
}

pub fn read_env_file(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(path).map_err(|e| io_error(path, e))?;
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

#[cfg(unix)]
fn restrict_env_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perm = fs::metadata(path)
        .map_err(|e| io_error(path, e))?
        .permissions();
    perm.set_mode(0o600);
    fs::set_permissions(path, perm).map_err(|e| io_error(path, e))
}

#[cfg(not(unix))]
fn restrict_env_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn io_error(path: &Path, source: io::Error) -> CoreError {
    CoreError::Io {
        path: path.display().to_string(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_preserves_comments_and_patches_keys() {
        let dir = std::env::temp_dir().join(format!("lumaway-core-env-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lumaway.env");
        fs::write(&path, "# comment\nLUMAWAY_BRIDGE=1.2.3.4\n").unwrap();

        upsert_env_file(
            &path,
            &[("LUMAWAY_BRIDGE", "10.0.0.2"), ("LUMAWAY_BRIDGE_ID", "abc")],
        )
        .unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# comment"));
        assert!(text.contains("LUMAWAY_BRIDGE=10.0.0.2"));
        assert!(text.contains("LUMAWAY_BRIDGE_ID=abc"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrate_adds_config_version_and_sync_mode_without_dropping_existing_keys() {
        let dir =
            std::env::temp_dir().join(format!("lumaway-core-env-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("lumaway.env");
        fs::write(
            &path,
            "# comment\nLUMAWAY_BRIDGE=1.2.3.4\nLUMAWAY_PRESET=desktop-wayland\n",
        )
        .unwrap();

        migrate_lumaway_env_v1(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# comment"));
        assert!(text.contains("LUMAWAY_BRIDGE=1.2.3.4"));
        assert!(text.contains("LUMAWAY_CONFIG_VERSION=1"));
        assert!(text.contains("LUMAWAY_SYNC_MODE=desktop"));

        let _ = fs::remove_dir_all(&dir);
    }
}
