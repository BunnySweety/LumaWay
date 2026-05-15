//! `calibrate-capture` command: probe CPU/GL backends and write a profile snippet.

use anyhow::{bail, Context, Result};
use lumaway_core::{CaptureBackend as CoreCaptureBackend, CaptureProfile, PortalScreenCast};
use tracing::info;

use crate::{
    calibrated_profile_text, probe_capture_backend, profile_path, recommended_backend,
    render_backend_recommendation, write_profile_file,
};

pub async fn run_calibrate_capture(
    name: &str,
    frames: u32,
    sample_width: i32,
    sample_height: i32,
    fps: i32,
    dark_threshold: u8,
    force: bool,
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
        .context("failed to duplicate PipeWire fd for CPU calibration probe")?;
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

    let backend = recommended_backend(&cpu, &gl);
    let path = profile_path(name)?;
    write_profile_file(
        &path,
        calibrated_profile_text(backend, sample_width, sample_height, &cpu, &gl),
        force,
    )?;

    println!("{}", cpu.render());
    println!("{}", gl.render());
    println!("{}", render_backend_recommendation(&cpu, &gl));
    println!(
        "calibrate_capture profile={} path={}",
        name.trim(),
        path.display()
    );
    Ok(())
}
