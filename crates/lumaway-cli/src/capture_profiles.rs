//! Parse comma-separated `WIDTHxHEIGHT` grids into capture profiles.

use anyhow::{Context, Result};
use lumaway_core::CaptureProfile;

pub fn parse_capture_profiles(grids: &str, fps: i32) -> Result<Vec<CaptureProfile>> {
    grids
        .split(',')
        .map(str::trim)
        .filter(|grid| !grid.is_empty())
        .map(|grid| parse_capture_profile(grid, fps))
        .collect()
}

fn parse_capture_profile(grid: &str, fps: i32) -> Result<CaptureProfile> {
    let (width, height) = grid
        .split_once('x')
        .or_else(|| grid.split_once('X'))
        .ok_or_else(|| anyhow::anyhow!("invalid grid '{grid}', expected WIDTHxHEIGHT"))?;
    let width = width
        .parse::<i32>()
        .with_context(|| format!("invalid grid width in '{grid}'"))?;
    let height = height
        .parse::<i32>()
        .with_context(|| format!("invalid grid height in '{grid}'"))?;
    CaptureProfile::new(width, height, fps).map_err(Into::into)
}
