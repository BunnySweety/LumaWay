//! CLI argument validation helpers shared across commands and presets.

use anyhow::{bail, Result};

pub fn validate_fps(name: &str, fps: u8) -> Result<()> {
    if fps == 0 {
        bail!("{name} must be greater than zero");
    }

    Ok(())
}

pub fn validate_capture_poll_ms(capture_poll_ms: u64) -> Result<()> {
    if capture_poll_ms == 0 {
        bail!("capture-poll-ms must be greater than zero");
    }
    if capture_poll_ms > 100 {
        bail!("capture-poll-ms must be lower than or equal to 100");
    }

    Ok(())
}

pub fn validate_brightness(brightness: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&brightness) {
        bail!("brightness must be between 0.0 and 1.0");
    }

    Ok(())
}
