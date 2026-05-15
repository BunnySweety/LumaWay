//! `sample-bench` command: multi-grid horizontal band timing.

use anyhow::{bail, Result};
use lumaway_core::{CaptureProfile, GStreamerTestCapture, PortalScreenCast};
use std::time::Duration;
use tracing::info;

use crate::{parse_capture_profiles, StageStats};

pub async fn run_sample_bench(
    portal: bool,
    frames: u32,
    bands: usize,
    grids: &str,
    fps: i32,
) -> Result<()> {
    if frames == 0 {
        bail!("frames must be greater than zero");
    }
    if bands == 0 {
        bail!("bands must be greater than zero");
    }

    let profiles = parse_capture_profiles(grids, fps)?;
    let first_profile = profiles
        .first()
        .copied()
        .ok_or_else(|| anyhow::anyhow!("at least one grid is required"))?;

    let capture = if portal {
        let mut selections = PortalScreenCast::select().await?;
        let selection = selections
            .pop()
            .ok_or_else(|| anyhow::anyhow!("portal returned no streams"))?;
        info!(
            node_id = selection.stream.pipewire_node_id,
            size = ?selection.stream.size,
            position = ?selection.stream.position,
            "selected portal stream"
        );
        GStreamerTestCapture::from_pipewire_node(
            selection.stream.pipewire_node_id,
            selection.pipewire_fd,
            first_profile.width,
            first_profile.height,
            first_profile.fps,
        )?
    } else {
        GStreamerTestCapture::new(first_profile.width, first_profile.height, first_profile.fps)?
    };

    capture.start()?;
    let result = run_sample_bench_loop(&capture, &profiles, frames, bands);
    let stop_result = capture.stop();

    match (result, stop_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}

fn run_sample_bench_loop(
    capture: &GStreamerTestCapture,
    profiles: &[CaptureProfile],
    frames: u32,
    bands: usize,
) -> Result<()> {
    let mut capture_stats = StageStats::default();
    let mut grid_stats = vec![StageStats::default(); profiles.len()];

    for _ in 0..frames {
        let frame = capture.benchmark_horizontal_averages_profiles(
            bands,
            Duration::from_secs(5),
            profiles,
        )?;
        capture_stats.record(frame.capture_duration);

        for (index, grid) in frame.grids.into_iter().enumerate() {
            grid_stats[index].record(grid.duration);
        }
    }

    for (profile, stats) in profiles.iter().zip(grid_stats.iter()) {
        println!(
            "sample_bench grid={}x{} frames={} bands={} capture_avg_ms={:.3} capture_max_ms={:.3} sample_avg_ms={:.3} sample_max_ms={:.3}",
            profile.width,
            profile.height,
            frames,
            bands,
            capture_stats.average_ms(),
            capture_stats.max_ms(),
            stats.average_ms(),
            stats.max_ms()
        );
    }

    Ok(())
}
