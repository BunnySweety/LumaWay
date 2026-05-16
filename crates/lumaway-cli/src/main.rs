use anyhow::Result;
use clap::Parser;

mod bench;
mod bridge_env;
mod calibrate_capture_cmd;
mod capture_loops;
mod capture_profiles;
mod capture_quality_cmd;
mod cli_args;
mod cli_defaults;
mod color_tuning;
mod crop;
mod detect_crop;
mod doctor;
mod hue_stream;
mod main_dispatch;
mod pipewire_capture;
mod presets;
mod profile_env;
mod sample_bench_cmd;
mod sample_debug_cmd;
mod sampling;
mod sync_run;
mod sync_stats;
mod tracing_init;
mod validation;

#[cfg(test)]
mod test_support;

pub use cli_defaults::{DEFAULT_AUTO_CROP_MAX_EDGE, DEFAULT_CAPTURE_POLL_MS};

pub use tracing_init::init_tracing;

pub use validation::{validate_brightness, validate_capture_poll_ms, validate_fps};

pub use hue_stream::{send_fixed_color, sleep_until_or_interrupt, SleepOutcome};

pub use pipewire_capture::{
    calibrated_profile_text, create_pipewire_capture, probe_capture_backend, recommended_backend,
    render_backend_recommendation, run_backend_probe, BackendProbeResult, CaptureBackend,
    BACKEND_AUTO_DARK_THRESHOLD,
};

pub use capture_loops::{capture_averages, run_capture_quality_loop, run_sample_debug_loop};

pub use sync_run::{run_sync, run_sync_bench};

pub use calibrate_capture_cmd::run_calibrate_capture;
pub use capture_quality_cmd::run_capture_quality;
pub use sample_bench_cmd::run_sample_bench;
pub use sample_debug_cmd::run_sample_debug;

pub use capture_profiles::parse_capture_profiles;
pub use detect_crop::run_detect_crop;

pub use crop::{
    cap_detected_crop, detect_crop_for_sync, max_detected_crop, sync_crop_args,
    validate_auto_crop_max_edge,
};

pub use profile_env::{
    available_profiles, config_home, default_profile_text, ensure_profile_loaded, home_dir,
    is_main_env_key, is_profile_key, list_profiles, load_main_env_defaults,
    load_profile_env_defaults, main_env_path, profile_path, read_key_value_file,
    write_profile_file, write_profile_template, PROFILE_LOAD_ERROR_ENV,
};

pub use color_tuning::{
    graded_color_from_average, hue_color_from_average, hue_color_from_graded, hue_luma,
    hue_saturation, rgb_luma, ColorProfile, ColorTuning,
};

pub use sync_stats::{capture_quality_hint, CaptureQualityStats, StageStats, SyncStats};

pub use sampling::{
    channel_samples_by_position, ChannelSample, SampleCrop, SamplingMode,
    DEFAULT_SAMPLE_EDGE_MARGIN,
};

pub use bench::{black, send_dtls_frame, synthetic_bench_area, NullTransport};

pub use presets::{
    deadline_reached, effective_capture_poll_timeout, effective_pipewire_fps, expected_frames,
    frame_delay_for_fps, parse_capture_poll_ms_list, resolve_preset_for_mode, SyncPreset,
    SyncPresetConfig,
};

pub use cli_args::{Cli, Command};

use main_dispatch::dispatch;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let _ = lumaway_core::migrate_lumaway_env_v1(&lumaway_core::lumaway_main_env_path());
    load_main_env_defaults();
    load_profile_env_defaults();

    let cli = Cli::parse();

    dispatch(cli.command).await
}

#[cfg(test)]
mod cli_bin_tests;
