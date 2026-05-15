//! Timing and capture-quality statistics for `sync` / `capture-quality`.

use std::fmt;
use std::time::Duration;

use lumaway_core::RgbAverage;

use crate::color_tuning::{graded_color_from_average, hue_saturation, rgb_luma, ColorTuning};
use crate::CaptureBackend;

#[derive(Debug, Default, Clone)]
pub struct SyncStats {
    pub capture_backend: Option<CaptureBackend>,
    pub interrupted: bool,
    pub frames: u64,
    pub capture_frames: u64,
    pub repeated_frames: u64,
    pub missed_capture_frames: u64,
    pub empty_capture_polls: u64,
    pub capture: StageStats,
    pub color: StageStats,
    pub encode: StageStats,
    pub send: StageStats,
}

impl fmt::Display for SyncStats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "sync_stats capture_backend={} interrupted={} frames={} capture_frames={} repeated_frames={} missed_capture_frames={} empty_capture_polls={} capture_avg_ms={:.3} capture_max_ms={:.3} color_avg_ms={:.3} color_max_ms={:.3} encode_avg_ms={:.3} encode_max_ms={:.3} send_avg_ms={:.3} send_max_ms={:.3}",
            self.capture_backend
                .map(|backend| backend.label())
                .unwrap_or("unknown"),
            self.interrupted,
            self.frames,
            self.capture_frames,
            self.repeated_frames,
            self.missed_capture_frames,
            self.empty_capture_polls,
            self.capture.average_ms(),
            self.capture.max_ms(),
            self.color.average_ms(),
            self.color.max_ms(),
            self.encode.average_ms(),
            self.encode.max_ms(),
            self.send.average_ms(),
            self.send.max_ms(),
        )
    }
}

#[derive(Debug, Default, Clone)]
pub struct StageStats {
    samples: u64,
    total: Duration,
    max: Duration,
}

impl StageStats {
    pub fn record(&mut self, duration: Duration) {
        self.samples += 1;
        self.total += duration;
        self.max = self.max.max(duration);
    }

    pub fn average_ms(&self) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }

        self.total.as_secs_f64() * 1000.0 / self.samples as f64
    }

    pub fn max_ms(&self) -> f64 {
        self.max.as_secs_f64() * 1000.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct CaptureQualityStats {
    pub frames: u32,
    pub channels: usize,
    pub dark_frames: u32,
    pub luma_total: f64,
    pub saturation_total: f64,
    pub color_samples: u64,
    pub frame_delta_total: f64,
    pub frame_delta_samples: u64,
    pub max_frame_delta: f64,
    pub channel_separation_total: f64,
    pub channel_separation_samples: u64,
}

impl CaptureQualityStats {
    pub fn record_frame(
        &mut self,
        averages: &[RgbAverage],
        previous: Option<&[RgbAverage]>,
        tuning: ColorTuning,
    ) {
        self.frames += 1;
        self.channels = averages.len();
        let mut max_rgb = 0;
        for average in averages {
            max_rgb = max_rgb
                .max(average.red)
                .max(average.green)
                .max(average.blue);
            self.luma_total += rgb_luma(*average);
            self.saturation_total += hue_saturation(graded_color_from_average(*average, tuning));
            self.color_samples += 1;
        }
        if max_rgb < crate::BACKEND_AUTO_DARK_THRESHOLD {
            self.dark_frames += 1;
        }

        if let Some(previous) = previous {
            for (current, previous) in averages.iter().zip(previous.iter()) {
                let delta = rgb_distance(*current, *previous);
                self.frame_delta_total += delta;
                self.frame_delta_samples += 1;
                self.max_frame_delta = self.max_frame_delta.max(delta);
            }
        }

        for (index, left) in averages.iter().enumerate() {
            for right in averages.iter().skip(index + 1) {
                self.channel_separation_total += rgb_distance(*left, *right);
                self.channel_separation_samples += 1;
            }
        }
    }

    pub fn avg_luma(&self) -> f64 {
        average_or_zero(self.luma_total, self.color_samples)
    }

    pub fn avg_saturation(&self) -> f64 {
        average_or_zero(self.saturation_total, self.color_samples)
    }

    pub fn avg_frame_delta(&self) -> f64 {
        average_or_zero(self.frame_delta_total, self.frame_delta_samples)
    }

    pub fn avg_channel_separation(&self) -> f64 {
        average_or_zero(
            self.channel_separation_total,
            self.channel_separation_samples,
        )
    }

    pub fn recommendation(&self) -> &'static str {
        if self.channels < 2 {
            "single_channel_area"
        } else if self.frames > 0 && self.dark_frames == self.frames {
            "capture_too_dark"
        } else if self.avg_frame_delta() < 2.0 {
            "low_temporal_variation"
        } else if self.avg_channel_separation() < 4.0 && self.channels > 1 {
            "low_spatial_separation"
        } else if self.avg_saturation() < 0.08 {
            "low_saturation"
        } else {
            "usable"
        }
    }

    pub fn warnings(&self) -> String {
        let mut warnings = Vec::new();
        if self.avg_luma() < 50.0 {
            warnings.push("low_luma");
        }
        if self.avg_saturation() < 0.08 {
            warnings.push("low_saturation");
        }
        if self.channels > 1 && self.avg_channel_separation() < 4.0 {
            warnings.push("low_spatial_separation");
        }
        if self.avg_frame_delta() < 2.0 {
            warnings.push("low_temporal_variation");
        }
        if warnings.is_empty() {
            "none".to_string()
        } else {
            warnings.join(",")
        }
    }
}

pub fn capture_quality_hint(recommendation: &str) -> &'static str {
    match recommendation {
        "single_channel_area" => "choose_multi_channel_area_for_correlation_test",
        "capture_too_dark" => "rerun_backend_probe_or_raise_brightness",
        "low_temporal_variation" => "test_with_moving_or_contrasting_windows",
        "low_spatial_separation" => "use_region_sampling_or_adjust_channel_regions",
        "low_saturation" => "try_boosted_color_profile",
        "usable" => "capture_is_usable_tune_color_profile_if_needed",
        _ => "inspect_sample_debug",
    }
}

fn average_or_zero(total: f64, samples: u64) -> f64 {
    if samples == 0 {
        0.0
    } else {
        total / samples as f64
    }
}

fn rgb_distance(left: RgbAverage, right: RgbAverage) -> f64 {
    let red = (f64::from(left.red) - f64::from(right.red)).abs();
    let green = (f64::from(left.green) - f64::from(right.green)).abs();
    let blue = (f64::from(left.blue) - f64::from(right.blue)).abs();
    (red + green + blue) / 3.0
}
