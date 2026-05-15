//! `sync` and `sync-bench`: portal capture, DTLS, and Hue streaming loop.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use lumaway_core::{
    ColorSmoother, ColorSmoothingConfig, CoreError, GStreamerTestCapture, PortalScreenCast,
};
use lumaway_hue::{
    resolve_dtls_psk_identity, ChannelColor, DtlsHueTransport, DtlsTransport, HueBridgeConfig,
    HueClient, HueStreamEncoder,
};
use tracing::{info, warn};

use crate::{
    black, cap_detected_crop, capture_averages, channel_samples_by_position,
    create_pipewire_capture, deadline_reached, detect_crop_for_sync,
    effective_capture_poll_timeout, effective_pipewire_fps, expected_frames, frame_delay_for_fps,
    hue_color_from_average, parse_capture_poll_ms_list, send_dtls_frame, sleep_until_or_interrupt,
    synthetic_bench_area, validate_auto_crop_max_edge, validate_brightness,
    validate_capture_poll_ms, validate_fps, CaptureBackend, ChannelSample, ColorProfile,
    ColorTuning, NullTransport, SampleCrop, SamplingMode, SleepOutcome, SyncPreset,
    SyncPresetConfig, SyncStats,
};

async fn connect_dtls_with_retries(
    bridge: &str,
    dtls_identity: String,
    client_key: String,
) -> lumaway_hue::Result<DtlsHueTransport> {
    const ATTEMPTS: usize = 3;
    const RETRY_DELAY: Duration = Duration::from_millis(350);

    let mut last_error = None;
    for attempt in 1..=ATTEMPTS {
        match DtlsHueTransport::connect(bridge, dtls_identity.clone(), client_key.as_str()) {
            Ok(transport) => return Ok(transport),
            Err(error) => {
                warn!(attempt, attempts = ATTEMPTS, %error, "DTLS connect attempt failed");
                last_error = Some(error);
                if attempt < ATTEMPTS {
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    Err(last_error.expect("DTLS retry loop should record an error"))
}

#[allow(clippy::too_many_arguments)]
async fn sync_average_color_loop(
    capture: &GStreamerTestCapture,
    transport: &mut impl DtlsTransport,
    area: &str,
    channel_samples: &[ChannelSample],
    duration_ms: u64,
    capture_fps: u8,
    stream_fps: u8,
    capture_poll_timeout: Duration,
    smoothing: f64,
    brightness: f64,
    noise_threshold: u8,
    max_step: Option<u8>,
    color_tuning: ColorTuning,
    sampling: SamplingMode,
) -> Result<SyncStats> {
    let capture_delay = frame_delay_for_fps(capture_fps.max(stream_fps));
    let stream_delay = frame_delay_for_fps(stream_fps);
    let mut sequence = 0u8;
    let mut stats = SyncStats::default();
    let points: Vec<_> = channel_samples.iter().map(|sample| sample.point).collect();
    let regions: Vec<_> = channel_samples.iter().map(|sample| sample.region).collect();
    let mut channels: Vec<_> = channel_samples
        .iter()
        .map(|sample| ChannelColor {
            channel_id: sample.channel_id,
            color: black(),
        })
        .collect();
    let mut encoder = HueStreamEncoder::new(area, channel_samples.len())?;
    let mut smoother = ColorSmoother::with_config(ColorSmoothingConfig {
        alpha: smoothing,
        noise_threshold,
        max_step,
    });

    let capture_started = Instant::now();
    let averages = capture_averages(capture, &points, &regions, sampling, Duration::from_secs(5))?;
    stats.capture.record(capture_started.elapsed());
    stats.capture_frames += 1;

    let max_raw = averages
        .iter()
        .map(|avg| avg.red.max(avg.green).max(avg.blue))
        .max()
        .unwrap_or(0u8);
    info!(
        channels = averages.len(),
        max_raw_channel_rgb = max_raw,
        sample = ?averages.first(),
        "first screen capture sample (before smoothing; if max_raw stays ~0, check auto-crop and PipeWire/portal)"
    );

    let color_started = Instant::now();
    let averages = smoother.smooth(averages);
    for (channel, average) in channels.iter_mut().zip(averages) {
        channel.color = hue_color_from_average(average, brightness, color_tuning);
    }
    stats.color.record(color_started.elapsed());

    let started_at = Instant::now();
    let deadline = if duration_ms == 0 {
        None
    } else {
        Some(started_at + Duration::from_millis(duration_ms))
    };
    let mut next_capture_at = started_at + capture_delay;
    let mut next_stream_at = started_at;
    let mut fresh_capture = true;

    while !deadline_reached(deadline) {
        if next_capture_at <= next_stream_at {
            if sleep_until_or_interrupt(next_capture_at).await? == SleepOutcome::Interrupted {
                stats.interrupted = true;
                break;
            }
            if deadline_reached(deadline) {
                break;
            }
            let capture_timeout = if stats.frames == 0 {
                Duration::from_secs(5)
            } else {
                effective_capture_poll_timeout(capture_poll_timeout, stream_delay)
            };
            let capture_started = Instant::now();
            match capture_averages(capture, &points, &regions, sampling, capture_timeout) {
                Ok(averages) => {
                    stats.capture.record(capture_started.elapsed());
                    stats.capture_frames += 1;
                    fresh_capture = true;

                    let color_started = Instant::now();
                    let averages = smoother.smooth(averages);
                    for (channel, average) in channels.iter_mut().zip(averages) {
                        channel.color = hue_color_from_average(average, brightness, color_tuning);
                    }
                    stats.color.record(color_started.elapsed());
                }
                Err(CoreError::CaptureTimeout) => {
                    stats.capture.record(capture_started.elapsed());
                    stats.empty_capture_polls += 1;
                }
                Err(error) => return Err(error.into()),
            }

            next_capture_at += capture_delay;
            while next_capture_at <= Instant::now() {
                next_capture_at += capture_delay;
            }
            continue;
        }

        if sleep_until_or_interrupt(next_stream_at).await? == SleepOutcome::Interrupted {
            stats.interrupted = true;
            break;
        }
        if deadline_reached(deadline) {
            break;
        }

        let encode_started = Instant::now();
        let message = encoder.encode_rgb(sequence, &channels);
        stats.encode.record(encode_started.elapsed());

        let send_started = Instant::now();
        send_dtls_frame(transport, message)?;
        stats.send.record(send_started.elapsed());

        sequence = sequence.wrapping_add(1);
        stats.frames += 1;
        if !fresh_capture {
            stats.repeated_frames += 1;
        }
        fresh_capture = false;
        next_stream_at += stream_delay;
        while next_stream_at <= Instant::now() {
            next_stream_at += stream_delay;
        }
    }

    let elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let expected_capture_frames = expected_frames(elapsed_ms, capture_fps);
    stats.missed_capture_frames = expected_capture_frames.saturating_sub(stats.capture_frames);
    Ok(stats)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_sync_bench(
    duration_ms: u64,
    capture_fps: u8,
    stream_fps: u8,
    pipewire_fps: Option<i32>,
    capture_backend: CaptureBackend,
    capture_poll_ms: &str,
    sample_width: i32,
    sample_height: i32,
    sample_edge_margin: f64,
    smoothing: f64,
    brightness: f64,
    color_profile: ColorProfile,
    noise_threshold: u8,
    max_step: Option<u8>,
) -> Result<()> {
    validate_fps("capture-fps", capture_fps)?;
    validate_fps("stream-fps", stream_fps)?;
    let pipewire_fps = effective_pipewire_fps(pipewire_fps, capture_fps, stream_fps)?;
    if !(0.0..=1.0).contains(&smoothing) {
        bail!("smoothing must be between 0.0 and 1.0");
    }
    validate_brightness(brightness)?;
    if !(0.0..0.5).contains(&sample_edge_margin) {
        bail!("sample-edge-margin must be greater than or equal to 0.0 and lower than 0.5");
    }
    let capture_poll_values = parse_capture_poll_ms_list(capture_poll_ms)?;

    info!(
        duration_ms,
        capture_fps,
        stream_fps,
        pipewire_fps,
        ?capture_backend,
        capture_poll_ms,
        sample_width,
        sample_height,
        sample_edge_margin,
        smoothing,
        brightness,
        noise_threshold,
        ?max_step,
        "sync-bench command requested"
    );

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

    let capture = create_pipewire_capture(
        selection.stream.pipewire_node_id,
        selection.pipewire_fd,
        sample_width,
        sample_height,
        pipewire_fps,
        capture_backend,
    )?;
    let effective_capture_backend = CaptureBackend::from(capture.backend());
    info!(
        ?capture_backend,
        ?effective_capture_backend,
        "capture backend selected"
    );
    if let Err(error) = capture.start() {
        return Err(error.into());
    }

    let area = synthetic_bench_area();
    let channel_samples =
        channel_samples_by_position(&area, sample_edge_margin, SampleCrop::default());

    let mut result = Ok(());
    for capture_poll_ms in capture_poll_values {
        let mut transport = NullTransport::default();
        let sync_result = sync_average_color_loop(
            &capture,
            &mut transport,
            &area.id,
            &channel_samples,
            duration_ms,
            capture_fps,
            stream_fps,
            Duration::from_millis(capture_poll_ms),
            smoothing,
            brightness,
            noise_threshold,
            max_step,
            ColorTuning::from(color_profile),
            SamplingMode::Point,
        )
        .await;
        let sync_result = sync_result.map(|mut stats| {
            stats.capture_backend = Some(effective_capture_backend);
            stats
        });

        match sync_result {
            Ok(stats) => {
                println!(
                    "bench_config capture_fps={} stream_fps={} capture_backend={:?} effective_capture_backend={:?} capture_poll_ms={} sent_bytes={}",
                    capture_fps,
                    stream_fps,
                    capture_backend,
                    effective_capture_backend,
                    capture_poll_ms,
                    transport.sent_bytes
                );
                println!("{stats}");
            }
            Err(error) => {
                result = Err(error);
                break;
            }
        }
    }

    let stop_result = capture.stop();
    match (result, stop_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run_sync(
    bridge: String,
    app_key: String,
    client_key: String,
    area: String,
    preset: Option<SyncPreset>,
    duration_ms: u64,
    fps: u8,
    capture_fps: Option<u8>,
    stream_fps: Option<u8>,
    pipewire_fps: Option<i32>,
    capture_backend: Option<CaptureBackend>,
    capture_poll_ms: u64,
    sample_width: i32,
    sample_height: i32,
    sample_edge_margin: f64,
    sampling: Option<SamplingMode>,
    sample_crop_left: f64,
    sample_crop_right: f64,
    sample_crop_top: f64,
    sample_crop_bottom: f64,
    auto_crop: bool,
    auto_crop_frames: u32,
    auto_crop_threshold: u8,
    auto_crop_max_edge: f64,
    smoothing: f64,
    brightness: f64,
    color_profile: ColorProfile,
    noise_threshold: u8,
    max_step: Option<u8>,
) -> Result<()> {
    let preset = preset.map(SyncPresetConfig::from);
    let capture_fps = capture_fps
        .or_else(|| preset.as_ref().map(|preset| preset.capture_fps))
        .unwrap_or(fps);
    let stream_fps = stream_fps
        .or_else(|| preset.as_ref().map(|preset| preset.stream_fps))
        .unwrap_or(fps);
    let pipewire_fps = pipewire_fps.or_else(|| preset.as_ref().map(|preset| preset.pipewire_fps));
    let capture_backend = capture_backend
        .or_else(|| preset.as_ref().map(|preset| preset.capture_backend))
        .unwrap_or(CaptureBackend::Cpu);
    let capture_poll_ms = preset
        .as_ref()
        .map(|preset| preset.capture_poll_ms)
        .unwrap_or(capture_poll_ms);
    let sampling = sampling
        .or_else(|| preset.as_ref().map(|preset| preset.sampling))
        .unwrap_or(SamplingMode::Point);
    let auto_crop = auto_crop || preset.as_ref().is_some_and(|preset| preset.auto_crop);
    let max_step = max_step.or_else(|| preset.as_ref().and_then(|preset| preset.max_step));
    validate_fps("fps", fps)?;
    validate_fps("capture-fps", capture_fps)?;
    validate_fps("stream-fps", stream_fps)?;
    let pipewire_fps = effective_pipewire_fps(pipewire_fps, capture_fps, stream_fps)?;
    validate_capture_poll_ms(capture_poll_ms)?;
    if fps != capture_fps || fps != stream_fps {
        info!(
            fps,
            capture_fps, stream_fps, "--fps overridden by explicit capture or stream cadence"
        );
    }
    if !(0.0..=1.0).contains(&smoothing) {
        bail!("smoothing must be between 0.0 and 1.0");
    }
    validate_brightness(brightness)?;
    if !(0.0..0.5).contains(&sample_edge_margin) {
        bail!("sample-edge-margin must be greater than or equal to 0.0 and lower than 0.5");
    }
    let sample_crop = SampleCrop::new(
        sample_crop_left,
        sample_crop_right,
        sample_crop_top,
        sample_crop_bottom,
    )?;
    if auto_crop && auto_crop_frames == 0 {
        bail!("auto-crop-frames must be greater than zero when auto-crop is enabled");
    }
    validate_auto_crop_max_edge(auto_crop_max_edge)?;

    info!(%bridge, %area, duration_ms, fps, capture_fps, stream_fps, pipewire_fps, ?capture_backend, capture_poll_ms, sample_width, sample_height, sample_edge_margin, ?sampling, sample_crop_left, sample_crop_right, sample_crop_top, sample_crop_bottom, auto_crop, auto_crop_frames, auto_crop_threshold, auto_crop_max_edge, smoothing, brightness, noise_threshold, ?max_step, "sync command requested");
    if duration_ms > 0 {
        info!(
            duration_ms,
            "sync will stop when this duration elapses (set 0 to run until interrupted)"
        );
    }
    let client = HueClient::new(HueBridgeConfig {
        bridge_ip: bridge.clone(),
        app_key: Some(app_key.clone()),
        client_key: Some(client_key.clone()),
    })?;
    let entertainment_area = client.resolve_entertainment_area(&area).await?;
    let entertainment_area_id = entertainment_area.id.clone();
    if entertainment_area.channels.is_empty() {
        bail!(
            "entertainment area \"{}\" ({}) has no channels — add lights to this entertainment zone in the Hue app, or pick another zone",
            entertainment_area.name,
            entertainment_area_id
        );
    }
    info!(
        channel_count = entertainment_area.channels.len(),
        "loaded entertainment area channels"
    );

    let dtls_identity = resolve_dtls_psk_identity(&client, app_key.as_str()).await?;
    let dtls_identity_env = std::env::var("LUMAWAY_DTLS_IDENTITY")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || matches!(
            std::env::var("LUMAWAY_DTLS_USE_APP_KEY").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
        );
    if dtls_identity_env {
        info!("DTLS PSK identity forced via environment (LUMAWAY_DTLS_IDENTITY / LUMAWAY_DTLS_USE_APP_KEY; see resolve_dtls_psk_identity)");
    }

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

    let capture = create_pipewire_capture(
        selection.stream.pipewire_node_id,
        selection.pipewire_fd,
        sample_width,
        sample_height,
        pipewire_fps,
        capture_backend,
    )?;
    let effective_capture_backend = CaptureBackend::from(capture.backend());
    info!(
        ?capture_backend,
        ?effective_capture_backend,
        "capture backend selected"
    );
    if let Err(error) = capture.start() {
        return Err(error.into());
    }

    let effective_crop = if auto_crop {
        let detected_crop = detect_crop_for_sync(&capture, auto_crop_frames, auto_crop_threshold)?;
        let detected_crop = cap_detected_crop(detected_crop, auto_crop_max_edge);
        let effective_crop = sample_crop.max_detected(detected_crop)?;
        println!(
            "auto_crop frames={} left={:.4} right={:.4} top={:.4} bottom={:.4}",
            auto_crop_frames,
            effective_crop.left,
            effective_crop.right,
            effective_crop.top,
            effective_crop.bottom
        );
        effective_crop
    } else {
        sample_crop
    };
    let channel_samples =
        channel_samples_by_position(&entertainment_area, sample_edge_margin, effective_crop);

    let hue_brightness_pct = (brightness * 100.0).clamp(1.0, 100.0);
    if let Err(error) = client
        .set_entertainment_area_lights(&entertainment_area_id, true, Some(hue_brightness_pct))
        .await
    {
        let _ = capture.stop();
        return Err(error.into());
    }
    if let Err(error) = client.activate_entertainment(&entertainment_area_id).await {
        let stop_result = capture.stop();
        let deactivate_result = client
            .deactivate_entertainment(&entertainment_area_id)
            .await;
        match (stop_result, deactivate_result) {
            (Ok(()), Ok(())) => return Err(error.into()),
            (Err(stop_error), Ok(())) => {
                return Err(error).context(format!("also failed to stop capture: {stop_error}"));
            }
            (Ok(()), Err(deactivate_error)) => {
                return Err(error)
                    .context(format!("also failed to deactivate: {deactivate_error}"));
            }
            (Err(stop_error), Err(deactivate_error)) => {
                return Err(error).context(format!(
                    "also failed to stop capture: {stop_error}; also failed to deactivate: {deactivate_error}"
                ));
            }
        }
    }
    if let Err(error) = client
        .set_entertainment_area_lights(&entertainment_area_id, true, Some(hue_brightness_pct))
        .await
    {
        let stop_result = capture.stop();
        let deactivate_result = client
            .deactivate_entertainment(&entertainment_area_id)
            .await;
        match (stop_result, deactivate_result) {
            (Ok(()), Ok(())) => return Err(error.into()),
            (Err(stop_error), Ok(())) => {
                return Err(error).context(format!("also failed to stop capture: {stop_error}"));
            }
            (Ok(()), Err(deactivate_error)) => {
                return Err(error)
                    .context(format!("also failed to deactivate: {deactivate_error}"));
            }
            (Err(stop_error), Err(deactivate_error)) => {
                return Err(error).context(format!(
                    "also failed to stop capture: {stop_error}; also failed to deactivate: {deactivate_error}"
                ));
            }
        }
    }
    info!(
        hue_brightness_pct,
        "entertainment zone lights set for streaming (before and after entertainment start)"
    );
    // Same ~500 ms pause as Lumux between REST activate and DTLS (bridge / streaming readiness).
    tokio::time::sleep(Duration::from_millis(500)).await;
    let mut transport = match connect_dtls_with_retries(&bridge, dtls_identity, client_key).await {
        Ok(transport) => transport,
        Err(error) => {
            let stop_result = capture.stop();
            let deactivate_result = client
                .deactivate_entertainment(&entertainment_area_id)
                .await;
            match (stop_result, deactivate_result) {
                (Ok(()), Ok(())) => return Err(error.into()),
                (Err(stop_error), Ok(())) => {
                    return Err(error)
                        .context(format!("also failed to stop capture: {stop_error}"));
                }
                (Ok(()), Err(deactivate_error)) => {
                    return Err(error)
                        .context(format!("also failed to deactivate: {deactivate_error}"));
                }
                (Err(stop_error), Err(deactivate_error)) => {
                    return Err(error).context(format!(
                        "also failed to stop capture: {stop_error}; also failed to deactivate: {deactivate_error}"
                    ));
                }
            }
        }
    };

    let sync_result = sync_average_color_loop(
        &capture,
        &mut transport,
        &entertainment_area_id,
        &channel_samples,
        duration_ms,
        capture_fps,
        stream_fps,
        Duration::from_millis(capture_poll_ms),
        smoothing,
        brightness,
        noise_threshold,
        max_step,
        ColorTuning::from(color_profile),
        sampling,
    )
    .await;
    let sync_result = sync_result.map(|mut stats| {
        stats.capture_backend = Some(effective_capture_backend);
        stats
    });

    let stop_result = capture.stop();
    let deactivate_result = client
        .deactivate_entertainment(&entertainment_area_id)
        .await;

    match (sync_result, stop_result, deactivate_result) {
        (Ok(stats), Ok(()), Ok(())) => {
            info!(frames_sent = stats.frames, "sent sync frames");
            if duration_ms > 0 && !stats.interrupted {
                info!(
                    duration_ms,
                    "sync stopped after requested duration; bridge entertainment was deactivated"
                );
            }
            println!("{stats}");
            Ok(())
        }
        (Err(error), _, _) => Err(error),
        (Ok(_), Err(error), _) => Err(error.into()),
        (Ok(_), Ok(()), Err(error)) => Err(error.into()),
    }
}
