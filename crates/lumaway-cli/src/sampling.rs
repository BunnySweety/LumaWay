//! Screen-space sampling for Hue channel anchors (crop, regions, ordering).

use anyhow::{bail, Result};
use clap::ValueEnum;
use lumaway_core::{DetectedSampleCrop, SamplePoint, SampleRegion};
use lumaway_hue::{EntertainmentArea, EntertainmentChannel, EntertainmentChannelPosition};

pub const DEFAULT_SAMPLE_EDGE_MARGIN: f64 = 0.08;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SamplingMode {
    Point,
    Region,
}

const POSITION_SPAN_EPSILON: f64 = 0.000_001;

#[derive(Debug, Clone, Copy)]
pub struct ChannelSample {
    pub channel_id: u8,
    pub point: SamplePoint,
    pub region: SampleRegion,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SampleCrop {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

impl SampleCrop {
    pub fn new(left: f64, right: f64, top: f64, bottom: f64) -> Result<Self> {
        for (name, value) in [
            ("sample-crop-left", left),
            ("sample-crop-right", right),
            ("sample-crop-top", top),
            ("sample-crop-bottom", bottom),
        ] {
            if !(0.0..1.0).contains(&value) {
                bail!("{name} must be greater than or equal to 0.0 and lower than 1.0");
            }
        }
        if left + right >= 1.0 {
            bail!("sample-crop-left plus sample-crop-right must be lower than 1.0");
        }
        if top + bottom >= 1.0 {
            bail!("sample-crop-top plus sample-crop-bottom must be lower than 1.0");
        }

        Ok(Self {
            left,
            right,
            top,
            bottom,
        })
    }

    pub fn apply(self, point: SamplePoint) -> SamplePoint {
        SamplePoint::new(
            self.left + point.x * (1.0 - self.left - self.right),
            self.top + point.y * (1.0 - self.top - self.bottom),
        )
    }

    pub fn max_detected(self, detected: DetectedSampleCrop) -> Result<Self> {
        Self::new(
            self.left.max(detected.left),
            self.right.max(detected.right),
            self.top.max(detected.top),
            self.bottom.max(detected.bottom),
        )
    }
}

pub fn channel_samples_by_position(
    area: &EntertainmentArea,
    edge_margin: f64,
    crop: SampleCrop,
) -> Vec<ChannelSample> {
    let ordered = channels_by_horizontal_position(area);
    let total = ordered.len().max(1);
    let x_range = position_axis_range(&ordered, |position| position.x);
    let y_range = position_axis_range(&ordered, |position| position.y);

    ordered
        .into_iter()
        .enumerate()
        .map(|(index, channel)| {
            let fallback_x = if total == 1 {
                0.5
            } else {
                index as f64 / (total - 1) as f64
            };
            let point = match channel.position {
                Some(position) => {
                    let x = x_range
                        .and_then(|(min, max)| normalize_position_axis(position.x, min, max))
                        .unwrap_or(0.5);
                    let y = y_range
                        .and_then(|(min, max)| normalize_position_axis(position.y, min, max))
                        .map(|normalized| 1.0 - normalized)
                        .unwrap_or(0.5);

                    crop.apply(SamplePoint::new(
                        margin_sample_axis(x, edge_margin),
                        margin_sample_axis(y, edge_margin),
                    ))
                }
                None => crop.apply(SamplePoint::new(
                    margin_sample_axis(fallback_x, edge_margin),
                    0.5,
                )),
            };

            ChannelSample {
                channel_id: channel.channel_id,
                point,
                region: sample_region_for_point(point),
            }
        })
        .collect()
}

fn sample_region_for_point(point: SamplePoint) -> SampleRegion {
    let horizontal_edge = (point.x - 0.5).abs();
    let vertical_edge = (point.y - 0.5).abs();
    let width = if horizontal_edge > vertical_edge {
        0.28
    } else {
        0.22
    };
    let height = if vertical_edge > horizontal_edge {
        0.28
    } else {
        0.22
    };
    SampleRegion::new(point, width, height)
}

fn position_axis_range(
    channels: &[EntertainmentChannel],
    axis: impl Fn(EntertainmentChannelPosition) -> f64,
) -> Option<(f64, f64)> {
    channels
        .iter()
        .filter_map(|channel| channel.position.map(&axis))
        .fold(None, |range, value| match range {
            Some((min, max)) => Some((min.min(value), max.max(value))),
            None => Some((value, value)),
        })
}

fn normalize_position_axis(value: f64, min: f64, max: f64) -> Option<f64> {
    let span = max - min;
    if span.abs() <= POSITION_SPAN_EPSILON {
        return None;
    }

    Some((value - min) / span)
}

fn margin_sample_axis(value: f64, edge_margin: f64) -> f64 {
    edge_margin + value.clamp(0.0, 1.0) * (1.0 - edge_margin * 2.0)
}

fn channels_by_horizontal_position(area: &EntertainmentArea) -> Vec<EntertainmentChannel> {
    let mut channels = area.channels.clone();
    channels.sort_by(|left, right| match (left.position, right.position) {
        (Some(left_position), Some(right_position)) => left_position
            .x
            .partial_cmp(&right_position.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.channel_id.cmp(&right.channel_id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.channel_id.cmp(&right.channel_id),
    });
    channels
}
