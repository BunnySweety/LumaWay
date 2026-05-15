//! PipeWire GStreamer capture, backend probing, and auto backend selection.

use std::os::fd::OwnedFd;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use lumaway_core::{
    CaptureBackend as CoreCaptureBackend, CaptureProfile, CoreError, GStreamerTestCapture,
    PortalScreenCast,
};
use tracing::{info, warn};

use crate::{default_profile_text, rgb_luma, StageStats};

const BACKEND_AUTO_PROBE_FRAMES: u32 = 3;
pub const BACKEND_AUTO_DARK_THRESHOLD: u8 = 8;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CaptureBackend {
    Auto,
    Cpu,
    Gl,
}

impl From<CoreCaptureBackend> for CaptureBackend {
    fn from(backend: CoreCaptureBackend) -> Self {
        match backend {
            CoreCaptureBackend::Cpu => Self::Cpu,
            CoreCaptureBackend::Gl => Self::Gl,
        }
    }
}

impl From<CaptureBackend> for CoreCaptureBackend {
    fn from(backend: CaptureBackend) -> Self {
        match backend {
            CaptureBackend::Auto => Self::Gl,
            CaptureBackend::Cpu => Self::Cpu,
            CaptureBackend::Gl => Self::Gl,
        }
    }
}

impl CaptureBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
            Self::Gl => "gl",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CaptureQuality {
    frames: u32,
    max_rgb: u8,
    avg_luma: f64,
    dark: bool,
}

impl CaptureQuality {
    fn usable(self) -> bool {
        self.frames > 0 && !self.dark
    }
}

fn capture_quality_loop(
    capture: &GStreamerTestCapture,
    frames: u32,
    dark_threshold: u8,
) -> std::result::Result<CaptureQuality, CoreError> {
    let mut luma_total = 0.0;
    let mut max_rgb = 0u8;
    let mut accepted_frames = 0u32;

    for _ in 0..frames {
        let color = capture.pull_average_color(Duration::from_secs(5))?;
        accepted_frames += 1;
        max_rgb = max_rgb.max(color.red).max(color.green).max(color.blue);
        luma_total += rgb_luma(color);
    }

    let avg_luma = if accepted_frames == 0 {
        0.0
    } else {
        luma_total / f64::from(accepted_frames)
    };

    Ok(CaptureQuality {
        frames: accepted_frames,
        max_rgb,
        avg_luma,
        dark: max_rgb <= dark_threshold || avg_luma <= f64::from(dark_threshold),
    })
}

fn validate_capture_quality(
    capture: &GStreamerTestCapture,
    frames: u32,
    dark_threshold: u8,
) -> std::result::Result<CaptureQuality, CoreError> {
    capture.start()?;
    let result = capture_quality_loop(capture, frames, dark_threshold);
    let stop_result = capture.stop();

    match (result, stop_result) {
        (Ok(quality), Ok(())) => Ok(quality),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

pub fn create_pipewire_capture(
    node_id: u32,
    pipewire_fd: OwnedFd,
    sample_width: i32,
    sample_height: i32,
    pipewire_fps: i32,
    backend: CaptureBackend,
) -> Result<GStreamerTestCapture> {
    match backend {
        CaptureBackend::Cpu | CaptureBackend::Gl => {
            GStreamerTestCapture::from_pipewire_node_with_backend(
                node_id,
                pipewire_fd,
                sample_width,
                sample_height,
                pipewire_fps,
                backend.into(),
            )
            .map_err(Into::into)
        }
        CaptureBackend::Auto => {
            let gl_fd = pipewire_fd
                .try_clone()
                .context("failed to duplicate PipeWire fd for GL backend fallback")?;
            match GStreamerTestCapture::from_pipewire_node_with_backend(
                node_id,
                gl_fd,
                sample_width,
                sample_height,
                pipewire_fps,
                CoreCaptureBackend::Gl,
            ) {
                Ok(capture) => match validate_capture_quality(
                    &capture,
                    BACKEND_AUTO_PROBE_FRAMES,
                    BACKEND_AUTO_DARK_THRESHOLD,
                ) {
                    Ok(quality) if quality.usable() => {
                        info!(
                            frames = quality.frames,
                            max_rgb = quality.max_rgb,
                            avg_luma = quality.avg_luma,
                            "GL capture backend passed auto quality probe"
                        );
                        Ok(capture)
                    }
                    Ok(quality) => {
                        warn!(
                            frames = quality.frames,
                            max_rgb = quality.max_rgb,
                            avg_luma = quality.avg_luma,
                            "GL capture backend returned dark/unusable frames; falling back to CPU backend"
                        );
                        GStreamerTestCapture::from_pipewire_node_with_backend(
                            node_id,
                            pipewire_fd,
                            sample_width,
                            sample_height,
                            pipewire_fps,
                            CoreCaptureBackend::Cpu,
                        )
                        .map_err(Into::into)
                    }
                    Err(error) => {
                        warn!(%error, "GL capture backend failed auto quality probe; falling back to CPU backend");
                        GStreamerTestCapture::from_pipewire_node_with_backend(
                            node_id,
                            pipewire_fd,
                            sample_width,
                            sample_height,
                            pipewire_fps,
                            CoreCaptureBackend::Cpu,
                        )
                        .map_err(Into::into)
                    }
                },
                Err(error) => {
                    warn!(%error, "GL capture backend unavailable; falling back to CPU backend");
                    GStreamerTestCapture::from_pipewire_node_with_backend(
                        node_id,
                        pipewire_fd,
                        sample_width,
                        sample_height,
                        pipewire_fps,
                        CoreCaptureBackend::Cpu,
                    )
                    .map_err(Into::into)
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct BackendProbeResult {
    pub backend: CaptureBackend,
    pub elapsed: Duration,
    pub requested_frames: u32,
    pub frames: u32,
    pub capture_avg_ms: f64,
    pub capture_max_ms: f64,
    pub max_rgb: u8,
    pub avg_luma: f64,
    pub dark: bool,
    pub error: Option<String>,
}

impl BackendProbeResult {
    fn error(backend: CoreCaptureBackend, elapsed: Duration, error: String) -> Self {
        Self {
            backend: CaptureBackend::from(backend),
            elapsed,
            requested_frames: 0,
            frames: 0,
            capture_avg_ms: 0.0,
            capture_max_ms: 0.0,
            max_rgb: 0,
            avg_luma: 0.0,
            dark: true,
            error: Some(error),
        }
    }

    fn usable(&self) -> bool {
        self.error.is_none() && self.frames > 0 && !self.dark
    }

    pub fn render(&self) -> String {
        match &self.error {
            Some(error) => format!(
                "backend_probe backend={} status=error elapsed_ms={:.3} error={}",
                self.backend.label(),
                self.elapsed.as_secs_f64() * 1000.0,
                shell_escape(error)
            ),
            None => format!(
                "backend_probe backend={} status=ok frames={}/{} dark={} max_rgb={} avg_luma={:.1} capture_avg_ms={:.3} capture_max_ms={:.3} elapsed_ms={:.3}",
                self.backend.label(),
                self.frames,
                self.requested_frames,
                self.dark,
                self.max_rgb,
                self.avg_luma,
                self.capture_avg_ms,
                self.capture_max_ms,
                self.elapsed.as_secs_f64() * 1000.0
            ),
        }
    }
}

pub fn probe_capture_backend(
    node_id: u32,
    pipewire_fd: OwnedFd,
    sample_width: i32,
    sample_height: i32,
    fps: i32,
    backend: CoreCaptureBackend,
    frames: u32,
    dark_threshold: u8,
) -> BackendProbeResult {
    let started = Instant::now();
    let capture = match GStreamerTestCapture::from_pipewire_node_with_backend(
        node_id,
        pipewire_fd,
        sample_width,
        sample_height,
        fps,
        backend,
    ) {
        Ok(capture) => capture,
        Err(error) => {
            return BackendProbeResult::error(backend, started.elapsed(), error.to_string());
        }
    };

    if let Err(error) = capture.start() {
        return BackendProbeResult::error(backend, started.elapsed(), error.to_string());
    }

    let mut capture_stats = StageStats::default();
    let mut luma_total = 0.0;
    let mut max_rgb = 0u8;
    let mut accepted_frames = 0u32;
    let mut error = None;

    for _ in 0..frames {
        let frame_started = Instant::now();
        match capture.pull_average_color(Duration::from_secs(5)) {
            Ok(color) => {
                capture_stats.record(frame_started.elapsed());
                accepted_frames += 1;
                max_rgb = max_rgb.max(color.red).max(color.green).max(color.blue);
                luma_total += rgb_luma(color);
            }
            Err(err) => {
                error = Some(err.to_string());
                break;
            }
        }
    }

    if let Err(err) = capture.stop() {
        error.get_or_insert_with(|| err.to_string());
    }

    let avg_luma = if accepted_frames == 0 {
        0.0
    } else {
        luma_total / f64::from(accepted_frames)
    };
    let dark = max_rgb <= dark_threshold || avg_luma <= f64::from(dark_threshold);

    BackendProbeResult {
        backend: CaptureBackend::from(backend),
        elapsed: started.elapsed(),
        requested_frames: frames,
        frames: accepted_frames,
        capture_avg_ms: capture_stats.average_ms(),
        capture_max_ms: capture_stats.max_ms(),
        max_rgb,
        avg_luma,
        dark,
        error,
    }
}

fn shell_escape(value: &str) -> String {
    let value = value.replace(['\n', '\r'], " ");
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn render_backend_recommendation(cpu: &BackendProbeResult, gl: &BackendProbeResult) -> String {
    let backend = recommended_backend(cpu, gl);
    let reason = if backend == "cpu" && cpu.usable() {
        "usable_stable_default"
    } else if backend == "gl" {
        "cpu_unusable_gl_usable"
    } else {
        "no_usable_backend_keep_stable_default"
    };

    format!("backend_probe_recommendation backend={backend} reason={reason}")
}

pub fn recommended_backend(cpu: &BackendProbeResult, gl: &BackendProbeResult) -> &'static str {
    if cpu.usable() {
        "cpu"
    } else if gl.usable() {
        "gl"
    } else {
        "cpu"
    }
}

pub fn calibrated_profile_text(
    capture_backend: &str,
    sample_width: i32,
    sample_height: i32,
    cpu: &BackendProbeResult,
    gl: &BackendProbeResult,
) -> String {
    let header = format!(
        "# Generated by lumaway calibrate-capture\n# cpu_frames={} cpu_dark={} cpu_max_rgb={} cpu_avg_luma={:.1}\n# gl_frames={} gl_dark={} gl_max_rgb={} gl_avg_luma={:.1}\n",
        cpu.frames,
        cpu.dark,
        cpu.max_rgb,
        cpu.avg_luma,
        gl.frames,
        gl.dark,
        gl.max_rgb,
        gl.avg_luma
    );
    format!(
        "{header}{}",
        default_profile_text(capture_backend, sample_width, sample_height)
    )
}

pub async fn run_backend_probe(
    frames: u32,
    sample_width: i32,
    sample_height: i32,
    fps: i32,
    dark_threshold: u8,
) -> Result<()> {
    if frames == 0 {
        bail!("frames must be greater than zero");
    }
    let _ = CaptureProfile::new(sample_width, sample_height, fps)?;

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

    let cpu_fd = selection
        .pipewire_fd
        .try_clone()
        .context("failed to duplicate PipeWire fd for CPU backend probe")?;
    let cpu = probe_capture_backend(
        selection.stream.pipewire_node_id,
        cpu_fd,
        sample_width,
        sample_height,
        fps,
        CoreCaptureBackend::Cpu,
        frames,
        dark_threshold,
    );
    let gl = probe_capture_backend(
        selection.stream.pipewire_node_id,
        selection.pipewire_fd,
        sample_width,
        sample_height,
        fps,
        CoreCaptureBackend::Gl,
        frames,
        dark_threshold,
    );

    println!("{}", cpu.render());
    println!("{}", gl.render());
    println!("{}", render_backend_recommendation(&cpu, &gl));
    Ok(())
}
