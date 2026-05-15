//! Pull RGB averages from a test capture and print quality / debug streams.

use std::time::{Duration, Instant};

use anyhow::Result;
use lumaway_core::{
    ColorSmoother, CoreError, GStreamerTestCapture, RgbAverage, SamplePoint, SampleRegion,
};
use lumaway_hue::EntertainmentArea;

use crate::{
    capture_quality_hint, graded_color_from_average, hue_color_from_graded, hue_luma,
    hue_saturation, rgb_luma, CaptureBackend, CaptureQualityStats, ChannelSample, ColorTuning,
    SamplingMode,
};

pub fn capture_averages(
    capture: &GStreamerTestCapture,
    points: &[SamplePoint],
    regions: &[SampleRegion],
    sampling: SamplingMode,
    timeout: Duration,
) -> std::result::Result<Vec<RgbAverage>, CoreError> {
    match sampling {
        SamplingMode::Point => capture.pull_point_averages(points, timeout),
        SamplingMode::Region => capture.pull_region_averages(regions, timeout),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn run_sample_debug_loop(
    capture: &GStreamerTestCapture,
    area: &EntertainmentArea,
    channel_samples: &[ChannelSample],
    points: &[SamplePoint],
    regions: &[SampleRegion],
    frames: u32,
    sampling: SamplingMode,
    capture_backend: CaptureBackend,
    radius_x: usize,
    radius_y: usize,
    smoother: &mut ColorSmoother,
    tuning: ColorTuning,
    brightness: f64,
) -> Result<()> {
    println!(
        "sample_debug_start area_id={} area_name={} channels={} capture_backend={} sampling={:?} radius_x={} radius_y={}",
        area.id,
        area.name,
        channel_samples.len(),
        capture_backend.label(),
        sampling,
        radius_x,
        radius_y
    );

    for frame in 1..=frames {
        let started = Instant::now();
        let raw = capture_averages(capture, points, regions, sampling, Duration::from_secs(5))?;
        let capture_ms = started.elapsed().as_secs_f64() * 1000.0;
        let smoothed = smoother.smooth(raw.clone());
        println!("sample_debug_frame frame={frame} capture_ms={capture_ms:.3}");

        for (sample, (raw, smoothed)) in channel_samples.iter().zip(raw.into_iter().zip(smoothed)) {
            let graded = graded_color_from_average(smoothed, tuning);
            let output = hue_color_from_graded(graded, brightness);
            println!(
                "sample_debug_channel frame={} channel={} point_x={:.4} point_y={:.4} region_width={:.3} region_height={:.3} radius_x={} radius_y={} raw_rgb={},{},{} smoothed_rgb={},{},{} graded_rgb={},{},{} output_rgb={},{},{} raw_luma={:.1} output_luma={:.1} output_saturation={:.3}",
                frame,
                sample.channel_id,
                sample.point.x,
                sample.point.y,
                sample.region.width,
                sample.region.height,
                radius_x,
                radius_y,
                raw.red,
                raw.green,
                raw.blue,
                smoothed.red,
                smoothed.green,
                smoothed.blue,
                graded.red,
                graded.green,
                graded.blue,
                output.red,
                output.green,
                output.blue,
                rgb_luma(raw),
                hue_luma(output),
                hue_saturation(output)
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn run_capture_quality_loop(
    capture: &GStreamerTestCapture,
    area: &EntertainmentArea,
    points: &[SamplePoint],
    regions: &[SampleRegion],
    frames: u32,
    sampling: SamplingMode,
    capture_backend: CaptureBackend,
    tuning: ColorTuning,
) -> Result<()> {
    let mut previous: Option<Vec<RgbAverage>> = None;
    let mut stats = CaptureQualityStats::default();
    for _ in 0..frames {
        let averages =
            capture_averages(capture, points, regions, sampling, Duration::from_secs(5))?;
        stats.record_frame(&averages, previous.as_deref(), tuning);
        previous = Some(averages);
    }

    let recommendation = stats.recommendation();
    println!(
        "capture_quality area_id={} area_name={} capture_backend={} sampling={:?} frames={} channels={} avg_luma={:.1} avg_saturation={:.3} avg_frame_delta={:.1} max_frame_delta={:.1} avg_channel_separation={:.1} dark_frames={} recommendation={} hint={} warnings={}",
        area.id,
        area.name,
        capture_backend.label(),
        sampling,
        stats.frames,
        stats.channels,
        stats.avg_luma(),
        stats.avg_saturation(),
        stats.avg_frame_delta(),
        stats.max_frame_delta,
        stats.avg_channel_separation(),
        stats.dark_frames,
        recommendation,
        capture_quality_hint(recommendation),
        stats.warnings()
    );
    Ok(())
}
