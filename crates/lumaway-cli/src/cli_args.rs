//! Clap CLI definitions for the `lumaway` binary.

use clap::{Parser, Subcommand};
use lumaway_core::SyncMode;

use crate::{
    CaptureBackend, ColorProfile, SamplingMode, SyncPreset, DEFAULT_AUTO_CROP_MAX_EDGE,
    DEFAULT_CAPTURE_POLL_MS, DEFAULT_SAMPLE_EDGE_MARGIN,
};

#[derive(Debug, Parser)]
#[command(name = "lumaway")]
#[command(about = "LumaWay headless CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    DiscoverBridges,
    ProfileList,
    ProfileTemplate {
        #[arg(long, default_value = "default")]
        name: String,
        #[arg(long)]
        force: bool,
    },
    CalibrateCapture {
        #[arg(long, default_value = "default")]
        name: String,
        #[arg(long, default_value_t = 5)]
        frames: u32,
        #[arg(long, default_value_t = 120)]
        sample_width: i32,
        #[arg(long, default_value_t = 68)]
        sample_height: i32,
        #[arg(long, default_value_t = 25)]
        fps: i32,
        #[arg(long, default_value_t = 8)]
        dark_threshold: u8,
        #[arg(long)]
        force: bool,
    },
    Auth {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
    },
    ListAreas {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
    },
    BridgeInfo {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
    },
    ActivateArea {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
        #[arg(long, default_value_t = 100.0)]
        brightness: f64,
    },
    DeactivateArea {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
    },
    TestColor {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_CLIENT_KEY", hide_env_values = true)]
        client_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
        #[arg(long)]
        color: String,
        #[arg(long, default_value_t = 2000)]
        duration_ms: u64,
        #[arg(long, default_value_t = 25)]
        fps: u8,
    },
    CaptureStats {
        #[arg(long, default_value_t = 2000)]
        duration_ms: u64,
        #[arg(long)]
        portal: bool,
        #[arg(long, default_value_t = 320)]
        width: i32,
        #[arg(long, default_value_t = 180)]
        height: i32,
        #[arg(long, default_value_t = 30)]
        fps: i32,
    },
    SampleBench {
        #[arg(long)]
        portal: bool,
        #[arg(long, default_value_t = 30)]
        frames: u32,
        #[arg(long, default_value_t = 2)]
        bands: usize,
        #[arg(long, default_value = "80x45,120x68,160x90,240x135")]
        grids: String,
        #[arg(long, default_value_t = 10)]
        fps: i32,
    },
    SampleDebug {
        #[arg(long)]
        portal: bool,
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
        #[arg(long, env = "LUMAWAY_SYNC_MODE")]
        sync_mode: Option<SyncMode>,
        #[arg(long, env = "LUMAWAY_PRESET", value_enum)]
        preset: Option<SyncPreset>,
        #[arg(long, default_value_t = 3)]
        frames: u32,
        #[arg(long, default_value_t = 25)]
        fps: u8,
        #[arg(long, env = "LUMAWAY_CAPTURE_FPS")]
        capture_fps: Option<u8>,
        #[arg(long, env = "LUMAWAY_PIPEWIRE_FPS")]
        pipewire_fps: Option<i32>,
        #[arg(long, env = "LUMAWAY_CAPTURE_BACKEND", value_enum)]
        capture_backend: Option<CaptureBackend>,
        #[arg(long, env = "LUMAWAY_SAMPLE_WIDTH", default_value_t = 120)]
        sample_width: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_HEIGHT", default_value_t = 68)]
        sample_height: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_EDGE_MARGIN", default_value_t = DEFAULT_SAMPLE_EDGE_MARGIN)]
        sample_edge_margin: f64,
        #[arg(long, env = "LUMAWAY_SAMPLING", value_enum)]
        sampling: Option<SamplingMode>,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_left: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_right: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_top: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_bottom: f64,
        #[arg(long, env = "LUMAWAY_REACTIVITY", default_value_t = 0.35)]
        smoothing: f64,
        #[arg(long, env = "LUMAWAY_BRIGHTNESS", default_value_t = 1.0)]
        brightness: f64,
        #[arg(long, env = "LUMAWAY_COLOR_PROFILE", value_enum)]
        color_profile: Option<ColorProfile>,
        #[arg(long, env = "LUMAWAY_NOISE_THRESHOLD", default_value_t = 3)]
        noise_threshold: u8,
        #[arg(long, env = "LUMAWAY_MAX_STEP")]
        max_step: Option<u8>,
    },
    CaptureQuality {
        #[arg(long)]
        portal: bool,
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
        #[arg(long, env = "LUMAWAY_SYNC_MODE")]
        sync_mode: Option<SyncMode>,
        #[arg(long, env = "LUMAWAY_PRESET", value_enum)]
        preset: Option<SyncPreset>,
        #[arg(long, default_value_t = 30)]
        frames: u32,
        #[arg(long, default_value_t = 25)]
        fps: u8,
        #[arg(long, env = "LUMAWAY_CAPTURE_FPS")]
        capture_fps: Option<u8>,
        #[arg(long, env = "LUMAWAY_PIPEWIRE_FPS")]
        pipewire_fps: Option<i32>,
        #[arg(long, env = "LUMAWAY_CAPTURE_BACKEND", value_enum)]
        capture_backend: Option<CaptureBackend>,
        #[arg(long, env = "LUMAWAY_SAMPLE_WIDTH", default_value_t = 120)]
        sample_width: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_HEIGHT", default_value_t = 68)]
        sample_height: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_EDGE_MARGIN", default_value_t = DEFAULT_SAMPLE_EDGE_MARGIN)]
        sample_edge_margin: f64,
        #[arg(long, env = "LUMAWAY_SAMPLING", value_enum)]
        sampling: Option<SamplingMode>,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_left: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_right: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_top: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_bottom: f64,
        #[arg(long, env = "LUMAWAY_COLOR_PROFILE", value_enum)]
        color_profile: Option<ColorProfile>,
    },
    DetectCrop {
        #[arg(long)]
        portal: bool,
        #[arg(long, default_value_t = 5)]
        frames: u32,
        #[arg(long, default_value_t = 120)]
        sample_width: i32,
        #[arg(long, default_value_t = 68)]
        sample_height: i32,
        #[arg(long, default_value_t = 10)]
        fps: i32,
        #[arg(long, default_value_t = 8)]
        threshold: u8,
        #[arg(long)]
        max_edge: Option<f64>,
    },
    BackendProbe {
        #[arg(long, default_value_t = 5)]
        frames: u32,
        #[arg(long, default_value_t = 120)]
        sample_width: i32,
        #[arg(long, default_value_t = 68)]
        sample_height: i32,
        #[arg(long, default_value_t = 25)]
        fps: i32,
        #[arg(long, default_value_t = 8)]
        dark_threshold: u8,
    },
    PortalProbe,
    Sync {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: String,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: String,
        #[arg(long, env = "LUMAWAY_CLIENT_KEY", hide_env_values = true)]
        client_key: String,
        #[arg(long, env = "LUMAWAY_AREA")]
        area: String,
        #[arg(long, env = "LUMAWAY_SYNC_MODE")]
        sync_mode: Option<SyncMode>,
        #[arg(long, env = "LUMAWAY_PRESET", value_enum)]
        preset: Option<SyncPreset>,
        #[arg(
            long,
            default_value_t = 0,
            help = "Sync duration in milliseconds; 0 runs until Ctrl-C / Stop (GUI default 0)"
        )]
        duration_ms: u64,
        #[arg(long, default_value_t = 25)]
        fps: u8,
        #[arg(long, env = "LUMAWAY_CAPTURE_FPS")]
        capture_fps: Option<u8>,
        #[arg(long, env = "LUMAWAY_STREAM_FPS")]
        stream_fps: Option<u8>,
        #[arg(long, env = "LUMAWAY_PIPEWIRE_FPS")]
        pipewire_fps: Option<i32>,
        #[arg(long, env = "LUMAWAY_CAPTURE_BACKEND", value_enum)]
        capture_backend: Option<CaptureBackend>,
        #[arg(long, env = "LUMAWAY_CAPTURE_POLL_MS", default_value_t = DEFAULT_CAPTURE_POLL_MS)]
        capture_poll_ms: u64,
        #[arg(long, env = "LUMAWAY_SAMPLE_WIDTH", default_value_t = 120)]
        sample_width: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_HEIGHT", default_value_t = 68)]
        sample_height: i32,
        #[arg(long, env = "LUMAWAY_SAMPLE_EDGE_MARGIN", default_value_t = DEFAULT_SAMPLE_EDGE_MARGIN)]
        sample_edge_margin: f64,
        #[arg(long, env = "LUMAWAY_SAMPLING", value_enum)]
        sampling: Option<SamplingMode>,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_left: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_right: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_top: f64,
        #[arg(long, default_value_t = 0.0)]
        sample_crop_bottom: f64,
        #[arg(long)]
        auto_crop: bool,
        #[arg(long, default_value_t = 5)]
        auto_crop_frames: u32,
        #[arg(long, default_value_t = 8)]
        auto_crop_threshold: u8,
        #[arg(long, default_value_t = DEFAULT_AUTO_CROP_MAX_EDGE)]
        auto_crop_max_edge: f64,
        #[arg(long, env = "LUMAWAY_REACTIVITY", default_value_t = 0.35)]
        smoothing: f64,
        #[arg(long, env = "LUMAWAY_BRIGHTNESS", default_value_t = 1.0)]
        brightness: f64,
        #[arg(long, env = "LUMAWAY_COLOR_PROFILE", value_enum)]
        color_profile: Option<ColorProfile>,
        #[arg(long, env = "LUMAWAY_NOISE_THRESHOLD", default_value_t = 3)]
        noise_threshold: u8,
        #[arg(long, env = "LUMAWAY_MAX_STEP")]
        max_step: Option<u8>,
    },
    SyncBench {
        #[arg(long, default_value_t = 10000)]
        duration_ms: u64,
        #[arg(long, default_value_t = 10)]
        capture_fps: u8,
        #[arg(long, default_value_t = 25)]
        stream_fps: u8,
        #[arg(long)]
        pipewire_fps: Option<i32>,
        #[arg(long, value_enum, default_value = "cpu")]
        capture_backend: CaptureBackend,
        #[arg(long, default_value = "5,8,12,20")]
        capture_poll_ms: String,
        #[arg(long, default_value_t = 120)]
        sample_width: i32,
        #[arg(long, default_value_t = 68)]
        sample_height: i32,
        #[arg(long, default_value_t = DEFAULT_SAMPLE_EDGE_MARGIN)]
        sample_edge_margin: f64,
        #[arg(long, default_value_t = 0.35)]
        smoothing: f64,
        #[arg(long, default_value_t = 1.0)]
        brightness: f64,
        #[arg(
            long,
            env = "LUMAWAY_COLOR_PROFILE",
            value_enum,
            default_value = "vivid"
        )]
        color_profile: ColorProfile,
        #[arg(long, default_value_t = 3)]
        noise_threshold: u8,
        #[arg(long)]
        max_step: Option<u8>,
    },
    Doctor {
        #[arg(long, env = "LUMAWAY_BRIDGE")]
        bridge: Option<String>,
        #[arg(long, env = "LUMAWAY_APP_KEY", hide_env_values = true)]
        app_key: Option<String>,
    },
}
