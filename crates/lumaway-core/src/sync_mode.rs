use std::str::FromStr;

pub const CONFIG_VERSION_KEY: &str = "LUMAWAY_CONFIG_VERSION";
pub const CURRENT_CONFIG_VERSION: &str = "1";
pub const SYNC_MODE_KEY: &str = "LUMAWAY_SYNC_MODE";
pub const LEGACY_PRESET_KEY: &str = "LUMAWAY_PRESET";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Video,
    Game,
    Desktop,
    Music,
}

impl SyncMode {
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Game => "game",
            Self::Desktop => "desktop",
            Self::Music => "music",
        }
    }

    pub fn default_preset(self) -> Option<&'static str> {
        match self {
            Self::Video => Some("video-wayland"),
            Self::Game => Some("game-wayland"),
            Self::Desktop => Some("desktop-wayland"),
            Self::Music => None,
        }
    }

    pub fn default_color_profile(self) -> &'static str {
        match self {
            Self::Video => "vivid",
            Self::Game => "game",
            Self::Desktop => "desktop",
            Self::Music => "music",
        }
    }

    pub fn from_preset_alias(value: &str) -> Option<Self> {
        match normalized(value).as_str() {
            "tv-wayland" | "video-wayland" => Some(Self::Video),
            "game-wayland" => Some(Self::Game),
            "desktop-wayland" => Some(Self::Desktop),
            _ => None,
        }
    }
}

impl std::fmt::Display for SyncMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_env_value())
    }
}

impl FromStr for SyncMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match normalized(value).as_str() {
            "video" => Ok(Self::Video),
            "game" => Ok(Self::Game),
            "desktop" => Ok(Self::Desktop),
            "music" => Ok(Self::Music),
            other => Err(format!(
                "unsupported sync mode `{other}`; expected video, game, desktop, or music"
            )),
        }
    }
}

pub fn resolve_sync_mode(sync_mode: Option<&str>, legacy_preset: Option<&str>) -> SyncMode {
    sync_mode
        .and_then(|value| value.parse::<SyncMode>().ok())
        .or_else(|| legacy_preset.and_then(SyncMode::from_preset_alias))
        .unwrap_or(SyncMode::Video)
}

pub fn config_v1_updates(
    values: &std::collections::HashMap<String, String>,
) -> Vec<(&'static str, String)> {
    let mut updates = Vec::new();
    if values.get(CONFIG_VERSION_KEY).map(|value| value.trim()) != Some(CURRENT_CONFIG_VERSION) {
        updates.push((CONFIG_VERSION_KEY, CURRENT_CONFIG_VERSION.to_string()));
    }
    if !values.contains_key(SYNC_MODE_KEY) {
        let mode = resolve_sync_mode(None, values.get(LEGACY_PRESET_KEY).map(String::as_str));
        updates.push((SYNC_MODE_KEY, mode.as_env_value().to_string()));
    }
    updates
}

fn normalized(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn parses_public_sync_modes() {
        assert_eq!("video".parse::<SyncMode>().unwrap(), SyncMode::Video);
        assert_eq!("game".parse::<SyncMode>().unwrap(), SyncMode::Game);
        assert_eq!("desktop".parse::<SyncMode>().unwrap(), SyncMode::Desktop);
        assert_eq!("music".parse::<SyncMode>().unwrap(), SyncMode::Music);
        assert!("scene".parse::<SyncMode>().is_err());
    }

    #[test]
    fn maps_legacy_presets_to_modes() {
        assert_eq!(
            SyncMode::from_preset_alias("tv-wayland"),
            Some(SyncMode::Video)
        );
        assert_eq!(
            SyncMode::from_preset_alias("video-wayland"),
            Some(SyncMode::Video)
        );
        assert_eq!(
            SyncMode::from_preset_alias("game-wayland"),
            Some(SyncMode::Game)
        );
        assert_eq!(
            SyncMode::from_preset_alias("desktop-wayland"),
            Some(SyncMode::Desktop)
        );
    }

    #[test]
    fn migration_adds_config_version_and_mode_from_legacy_preset() {
        let mut values = HashMap::new();
        values.insert(LEGACY_PRESET_KEY.to_string(), "game-wayland".to_string());
        let updates = config_v1_updates(&values);
        assert!(updates.contains(&(CONFIG_VERSION_KEY, CURRENT_CONFIG_VERSION.to_string())));
        assert!(updates.contains(&(SYNC_MODE_KEY, "game".to_string())));
    }
}
