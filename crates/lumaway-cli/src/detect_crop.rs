//! `detect-crop` command: test capture and black-bar detection.

use std::time::Duration;

use anyhow::{bail, Result};
use lumaway_core::{CaptureProfile, DetectedSampleCrop, GStreamerTestCapture, PortalScreenCast};
use tracing::info;

use crate::{cap_detected_crop, max_detected_crop, sync_crop_args, validate_auto_crop_max_edge};

pub async fn run_detect_crop(
    portal: bool,
    frames: u32,
    sample_width: i32,
    sample_height: i32,
    fps: i32,
    threshold: u8,
    max_edge: Option<f64>,
) -> Result<()> {
    if frames == 0 {
        bail!("frames must be greater than zero");
    }
    if let Some(max_edge) = max_edge {
        validate_auto_crop_max_edge(max_edge)?;
    }

    let profile = CaptureProfile::new(sample_width, sample_height, fps)?;
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
            profile.width,
            profile.height,
            profile.fps,
        )?
    } else {
        GStreamerTestCapture::new(profile.width, profile.height, profile.fps)?
    };

    capture.start()?;
    let result = run_detect_crop_loop(&capture, frames, threshold, max_edge);
    let stop_result = capture.stop();

    match (result, stop_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}

fn run_detect_crop_loop(
    capture: &GStreamerTestCapture,
    frames: u32,
    threshold: u8,
    max_edge: Option<f64>,
) -> Result<()> {
    let mut suggested = DetectedSampleCrop::NONE;

    for frame in 0..frames {
        let crop = capture.pull_detected_black_bars(threshold, Duration::from_secs(5))?;
        suggested = max_detected_crop(suggested, crop);
        println!(
            "crop_frame frame={} left={:.4} right={:.4} top={:.4} bottom={:.4}",
            frame + 1,
            crop.left,
            crop.right,
            crop.top,
            crop.bottom
        );
    }

    let effective = max_edge
        .map(|max_edge| cap_detected_crop(suggested, max_edge))
        .unwrap_or(suggested);
    println!(
        "crop_suggested frames={} left={:.4} right={:.4} top={:.4} bottom={:.4}",
        frames, suggested.left, suggested.right, suggested.top, suggested.bottom
    );
    if let Some(max_edge) = max_edge {
        println!(
            "crop_capped max_edge={:.4} left={:.4} right={:.4} top={:.4} bottom={:.4}",
            max_edge, effective.left, effective.right, effective.top, effective.bottom
        );
    }
    println!("{}", sync_crop_args(effective));
    Ok(())
}
