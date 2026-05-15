//! Sync presets (`video-wayland`, `game-wayland`, `desktop-wayland`) and shared timing helpers.

use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use std::time::{Duration, Instant};

use crate::{CaptureBackend, SamplingMode};
use lumaway_core::SyncMode;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SyncPreset {
    VideoWayland,
    GameWayland,
    DesktopWayland,
    TvWayland,
}

impl SyncPreset {
    pub fn from_sync_mode(mode: SyncMode) -> Option<Self> {
        match mode {
            SyncMode::Video => Some(Self::VideoWayland),
            SyncMode::Game => Some(Self::GameWayland),
            SyncMode::Desktop => Some(Self::DesktopWayland),
            SyncMode::Music => None,
        }
    }

    pub fn sync_mode(self) -> SyncMode {
        match self {
            Self::VideoWayland | Self::TvWayland => SyncMode::Video,
            Self::GameWayland => SyncMode::Game,
            Self::DesktopWayland => SyncMode::Desktop,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyncPresetConfig {
    pub capture_fps: u8,
    pub stream_fps: u8,
    pub pipewire_fps: i32,
    pub capture_backend: CaptureBackend,
    pub capture_poll_ms: u64,
    pub sampling: SamplingMode,
    pub auto_crop: bool,
    pub max_step: Option<u8>,
}

impl From<SyncPreset> for SyncPresetConfig {
    fn from(preset: SyncPreset) -> Self {
        match preset {
            SyncPreset::VideoWayland | SyncPreset::TvWayland => Self {
                capture_fps: 8,
                stream_fps: 25,
                pipewire_fps: 25,
                capture_backend: CaptureBackend::Cpu,
                capture_poll_ms: 5,
                sampling: SamplingMode::Region,
                // Letterbox auto-crop can mis-detect on some captures and shift samples into black bars;
                // GUI uses this preset only — default off; use CLI `--auto-crop` when needed.
                auto_crop: false,
                max_step: None,
            },
            SyncPreset::GameWayland => Self {
                capture_fps: 12,
                stream_fps: 25,
                pipewire_fps: 25,
                capture_backend: CaptureBackend::Cpu,
                capture_poll_ms: 5,
                sampling: SamplingMode::Region,
                auto_crop: false,
                max_step: None,
            },
            SyncPreset::DesktopWayland => Self {
                capture_fps: 6,
                stream_fps: 25,
                pipewire_fps: 25,
                capture_backend: CaptureBackend::Cpu,
                capture_poll_ms: 5,
                sampling: SamplingMode::Region,
                auto_crop: false,
                max_step: None,
            },
        }
    }
}

pub fn resolve_preset_for_mode(
    sync_mode: Option<SyncMode>,
    preset: Option<SyncPreset>,
) -> Option<SyncPreset> {
    sync_mode.and_then(SyncPreset::from_sync_mode).or(preset)
}

pub fn effective_pipewire_fps(
    pipewire_fps: Option<i32>,
    capture_fps: u8,
    stream_fps: u8,
) -> Result<i32> {
    let fps = pipewire_fps.unwrap_or_else(|| i32::from(capture_fps.max(stream_fps)));
    if fps <= 0 {
        bail!("pipewire-fps must be greater than zero");
    }

    Ok(fps)
}

pub fn parse_capture_poll_ms_list(values: &str) -> Result<Vec<u64>> {
    let parsed: Vec<u64> = values
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let capture_poll_ms = value
                .parse::<u64>()
                .with_context(|| format!("invalid capture poll value: {value}"))?;
            crate::validate_capture_poll_ms(capture_poll_ms)?;
            Ok(capture_poll_ms)
        })
        .collect::<Result<Vec<_>>>()?;

    if parsed.is_empty() {
        bail!("at least one capture poll value is required");
    }

    Ok(parsed)
}

pub fn frame_delay_for_fps(fps: u8) -> Duration {
    Duration::from_micros(1_000_000 / u64::from(fps))
}

pub fn expected_frames(duration_ms: u64, fps: u8) -> u64 {
    duration_ms.saturating_mul(u64::from(fps)) / 1000
}

pub fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

pub fn effective_capture_poll_timeout(configured: Duration, stream_delay: Duration) -> Duration {
    configured.max(stream_delay / 2)
}
