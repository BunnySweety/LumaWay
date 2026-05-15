//! `capture-quality` command: correlation / darkness heuristics on live capture.

use anyhow::{bail, Result};
use lumaway_core::{GStreamerTestCapture, PortalScreenCast};
use lumaway_hue::{HueBridgeConfig, HueClient};
use tracing::info;

use crate::{
    channel_samples_by_position, create_pipewire_capture, effective_pipewire_fps,
    run_capture_quality_loop, validate_fps, CaptureBackend, ColorProfile, ColorTuning, SampleCrop,
    SamplingMode, SyncPreset, SyncPresetConfig,
};

#[allow(clippy::too_many_arguments)]
pub async fn run_capture_quality(
    portal: bool,
    bridge: String,
    app_key: String,
    area: String,
    preset: Option<SyncPreset>,
    frames: u32,
    fps: u8,
    capture_fps: Option<u8>,
    pipewire_fps: Option<i32>,
    capture_backend: Option<CaptureBackend>,
    sample_width: i32,
    sample_height: i32,
    sample_edge_margin: f64,
    sampling: Option<SamplingMode>,
    sample_crop_left: f64,
    sample_crop_right: f64,
    sample_crop_top: f64,
    sample_crop_bottom: f64,
    color_profile: ColorProfile,
) -> Result<()> {
    if frames == 0 {
        bail!("frames must be greater than zero");
    }
    validate_fps("fps", fps)?;
    if !(0.0..0.5).contains(&sample_edge_margin) {
        bail!("sample-edge-margin must be greater than or equal to 0.0 and lower than 0.5");
    }

    let preset = preset.map(SyncPresetConfig::from);
    let capture_fps = capture_fps
        .or_else(|| preset.as_ref().map(|preset| preset.capture_fps))
        .unwrap_or(fps);
    validate_fps("capture-fps", capture_fps)?;
    let pipewire_fps = effective_pipewire_fps(
        pipewire_fps.or_else(|| preset.as_ref().map(|p| p.pipewire_fps)),
        capture_fps,
        capture_fps,
    )?;
    let capture_backend = capture_backend
        .or_else(|| preset.as_ref().map(|preset| preset.capture_backend))
        .unwrap_or(CaptureBackend::Cpu);
    let sampling = sampling
        .or_else(|| preset.as_ref().map(|preset| preset.sampling))
        .unwrap_or(SamplingMode::Region);
    let sample_crop = SampleCrop::new(
        sample_crop_left,
        sample_crop_right,
        sample_crop_top,
        sample_crop_bottom,
    )?;

    let client = HueClient::new(HueBridgeConfig {
        bridge_ip: bridge,
        app_key: Some(app_key),
        client_key: None,
    })?;
    let entertainment_area = client.resolve_entertainment_area(&area).await?;
    if entertainment_area.channels.is_empty() {
        bail!(
            "entertainment area \"{}\" ({}) has no channels",
            entertainment_area.name,
            entertainment_area.id
        );
    }

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
        create_pipewire_capture(
            selection.stream.pipewire_node_id,
            selection.pipewire_fd,
            sample_width,
            sample_height,
            pipewire_fps,
            capture_backend,
        )?
    } else {
        GStreamerTestCapture::new(sample_width, sample_height, i32::from(capture_fps))?
    };
    let effective_capture_backend = CaptureBackend::from(capture.backend());
    let channel_samples =
        channel_samples_by_position(&entertainment_area, sample_edge_margin, sample_crop);
    let points: Vec<_> = channel_samples.iter().map(|sample| sample.point).collect();
    let regions: Vec<_> = channel_samples.iter().map(|sample| sample.region).collect();
    let tuning = ColorTuning::from(color_profile);

    capture.start()?;
    let result = run_capture_quality_loop(
        &capture,
        &entertainment_area,
        &points,
        &regions,
        frames,
        sampling,
        effective_capture_backend,
        tuning,
    );
    let stop_result = capture.stop();

    match (result, stop_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}
