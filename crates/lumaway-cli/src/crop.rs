//! Detected letterbox crop helpers for `sync` / `detect-crop`.

use std::time::Duration;

use anyhow::{bail, Result};
use lumaway_core::{CoreError, DetectedSampleCrop, GStreamerTestCapture};
use tracing::warn;

pub fn sync_crop_args(crop: DetectedSampleCrop) -> String {
    format!(
        "crop_args --sample-crop-left {:.4} --sample-crop-right {:.4} --sample-crop-top {:.4} --sample-crop-bottom {:.4}",
        crop.left, crop.right, crop.top, crop.bottom
    )
}

pub fn validate_auto_crop_max_edge(max_edge: f64) -> Result<()> {
    if !(0.0..0.5).contains(&max_edge) {
        bail!("auto-crop-max-edge must be greater than or equal to 0.0 and lower than 0.5");
    }

    Ok(())
}

pub fn cap_detected_crop(crop: DetectedSampleCrop, max_edge: f64) -> DetectedSampleCrop {
    DetectedSampleCrop {
        left: crop.left.min(max_edge),
        right: crop.right.min(max_edge),
        top: crop.top.min(max_edge),
        bottom: crop.bottom.min(max_edge),
    }
}

pub fn max_detected_crop(
    left: DetectedSampleCrop,
    right: DetectedSampleCrop,
) -> DetectedSampleCrop {
    DetectedSampleCrop {
        left: left.left.max(right.left),
        right: left.right.max(right.right),
        top: left.top.max(right.top),
        bottom: left.bottom.max(right.bottom),
    }
}

pub fn detect_crop_for_sync(
    capture: &GStreamerTestCapture,
    frames: u32,
    threshold: u8,
) -> Result<DetectedSampleCrop> {
    let mut detected = DetectedSampleCrop::NONE;
    for frame in 0..frames {
        match capture.pull_detected_black_bars(threshold, Duration::from_secs(5)) {
            Ok(crop) => {
                detected = max_detected_crop(detected, crop);
            }
            Err(CoreError::CaptureTimeout) => {
                warn!(
                    frame = frame + 1,
                    frames,
                    "auto-crop timed out waiting for a frame; continuing with detected crop so far"
                );
                break;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(detected)
}
