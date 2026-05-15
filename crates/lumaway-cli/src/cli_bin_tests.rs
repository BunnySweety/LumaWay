//! Unit tests for the `lumaway` CLI binary.

use crate::{
    available_profiles, cap_detected_crop, capture_quality_hint, channel_samples_by_position,
    effective_capture_poll_timeout, effective_pipewire_fps, expected_frames, frame_delay_for_fps,
    hue_color_from_average, hue_luma, hue_saturation, is_profile_key, max_detected_crop,
    parse_capture_poll_ms_list, profile_path, send_dtls_frame, sync_crop_args,
    synthetic_bench_area, validate_auto_crop_max_edge, validate_brightness,
    validate_capture_poll_ms, validate_fps, BackendProbeResult, CaptureBackend,
    CaptureQualityStats, ColorProfile, ColorTuning, SampleCrop, SamplingMode, StageStats,
    SyncPreset, SyncPresetConfig, SyncStats, DEFAULT_SAMPLE_EDGE_MARGIN,
};
use lumaway_core::{DetectedSampleCrop, RgbAverage, SamplePoint};
use lumaway_hue::{
    DtlsTransport, EntertainmentArea, EntertainmentChannel, EntertainmentChannelPosition,
    HueStreamMessage,
};
use std::time::Duration;

#[derive(Default)]
struct RecordingTransport {
    sent: Vec<Vec<u8>>,
    drains: usize,
}

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

impl DtlsTransport for RecordingTransport {
    fn send(&mut self, message: &HueStreamMessage) -> lumaway_hue::Result<()> {
        self.send_bytes(message.as_bytes())
    }

    fn send_bytes(&mut self, bytes: &[u8]) -> lumaway_hue::Result<()> {
        self.sent.push(bytes.to_vec());
        Ok(())
    }

    fn drain_incoming(&mut self) -> lumaway_hue::Result<()> {
        self.drains += 1;
        Ok(())
    }
}

#[test]
fn dtls_frame_send_drains_peer_records() {
    let mut transport = RecordingTransport::default();

    send_dtls_frame(&mut transport, b"frame").unwrap();

    assert_eq!(transport.sent, vec![b"frame".to_vec()]);
    assert_eq!(transport.drains, 1);
}

#[test]
fn profile_path_rejects_path_traversal_names() {
    assert!(profile_path("").is_err());
    assert!(profile_path("../tv").is_err());
    assert!(profile_path("room/tv").is_err());
    assert!(profile_path("..").is_err());
    assert!(profile_path("tv")
        .unwrap()
        .ends_with("lumaway/profiles/tv.env"));
}

#[test]
fn profile_keys_are_limited_to_non_secret_capture_settings() {
    assert!(is_profile_key("LUMAWAY_CAPTURE_BACKEND"));
    assert!(is_profile_key("LUMAWAY_COLOR_PROFILE"));
    assert!(!is_profile_key("LUMAWAY_APP_KEY"));
    assert!(!is_profile_key("LUMAWAY_CLIENT_KEY"));
}

#[test]
fn available_profiles_returns_sorted_env_files_only() {
    let root = std::env::temp_dir().join(format!("lumaway-profile-test-{}", std::process::id()));
    let profiles_dir = root.join("lumaway/profiles");
    std::fs::create_dir_all(&profiles_dir).unwrap();
    std::fs::write(profiles_dir.join("zeta.env"), "").unwrap();
    std::fs::write(profiles_dir.join("alpha.env"), "").unwrap();
    std::fs::write(profiles_dir.join("README.txt"), "").unwrap();

    let old_config = std::env::var_os("XDG_CONFIG_HOME");
    std::env::set_var("XDG_CONFIG_HOME", &root);
    let profiles = available_profiles().unwrap();
    match old_config {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    let _ = std::fs::remove_dir_all(root);

    let names: Vec<_> = profiles.into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["alpha", "zeta"]);
}

#[test]
fn main_env_defaults_load_cli_and_profile_values_without_overrides() {
    let root = std::env::temp_dir().join(format!("lumaway-main-env-test-{}", std::process::id()));
    let config_dir = root.join("lumaway");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("lumaway.env"),
        "LUMAWAY_BRIDGE=192.0.2.10\nLUMAWAY_APP_KEY=file-key\nLUMAWAY_PROFILE=default\nIGNORED=value\n",
    )
    .unwrap();

    let old_config = std::env::var_os("XDG_CONFIG_HOME");
    let old_bridge = std::env::var_os("LUMAWAY_BRIDGE");
    let old_app_key = std::env::var_os("LUMAWAY_APP_KEY");
    let old_profile = std::env::var_os("LUMAWAY_PROFILE");
    std::env::set_var("XDG_CONFIG_HOME", &root);
    std::env::set_var("LUMAWAY_BRIDGE", "env-bridge");
    std::env::remove_var("LUMAWAY_APP_KEY");
    std::env::remove_var("LUMAWAY_PROFILE");

    crate::load_main_env_defaults();

    assert_eq!(std::env::var("LUMAWAY_BRIDGE").unwrap(), "192.0.2.10");
    assert_eq!(std::env::var("LUMAWAY_APP_KEY").unwrap(), "file-key");
    assert_eq!(std::env::var("LUMAWAY_PROFILE").unwrap(), "default");
    assert!(std::env::var_os("IGNORED").is_none());

    match old_config {
        Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }
    restore_env("LUMAWAY_BRIDGE", old_bridge);
    restore_env("LUMAWAY_APP_KEY", old_app_key);
    restore_env("LUMAWAY_PROFILE", old_profile);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn non_empty_env_trims_and_ignores_empty_values() {
    let old_bridge = std::env::var_os("LUMAWAY_BRIDGE");
    std::env::set_var("LUMAWAY_BRIDGE", " 192.0.2.10 ");
    assert_eq!(
        crate::doctor::non_empty_env("LUMAWAY_BRIDGE").as_deref(),
        Some("192.0.2.10")
    );

    std::env::set_var("LUMAWAY_BRIDGE", "   ");
    assert_eq!(crate::doctor::non_empty_env("LUMAWAY_BRIDGE"), None);

    restore_env("LUMAWAY_BRIDGE", old_bridge);
}

#[test]
fn calibrated_profile_records_backend_probe_result() {
    let cpu = BackendProbeResult {
        backend: CaptureBackend::Cpu,
        elapsed: Duration::from_millis(10),
        requested_frames: 3,
        frames: 3,
        capture_avg_ms: 3.0,
        capture_max_ms: 4.0,
        max_rgb: 120,
        avg_luma: 80.0,
        dark: false,
        error: None,
    };
    let gl = BackendProbeResult {
        backend: CaptureBackend::Gl,
        elapsed: Duration::from_millis(10),
        requested_frames: 3,
        frames: 3,
        capture_avg_ms: 3.0,
        capture_max_ms: 4.0,
        max_rgb: 0,
        avg_luma: 0.0,
        dark: true,
        error: None,
    };

    let text = crate::calibrated_profile_text("cpu", 120, 68, &cpu, &gl);

    assert!(text.contains("# cpu_frames=3 cpu_dark=false cpu_max_rgb=120"));
    assert!(text.contains("# gl_frames=3 gl_dark=true gl_max_rgb=0"));
    assert!(text.contains("LUMAWAY_CAPTURE_BACKEND=cpu"));
    assert!(text.contains("LUMAWAY_SAMPLING=region"));
}

#[test]
fn capture_quality_recommends_from_measured_stats() {
    let tuning = ColorTuning::from(ColorProfile::Vivid);
    let mut dark = CaptureQualityStats::default();
    dark.record_frame(
        &[
            RgbAverage {
                red: 0,
                green: 0,
                blue: 0,
            },
            RgbAverage {
                red: 0,
                green: 0,
                blue: 0,
            },
        ],
        None,
        tuning,
    );
    assert_eq!(dark.recommendation(), "capture_too_dark");

    let mut single = CaptureQualityStats::default();
    single.record_frame(
        &[RgbAverage {
            red: 80,
            green: 20,
            blue: 20,
        }],
        None,
        tuning,
    );
    assert_eq!(single.recommendation(), "single_channel_area");

    let mut flat = CaptureQualityStats::default();
    let frame = vec![
        RgbAverage {
            red: 80,
            green: 80,
            blue: 80,
        },
        RgbAverage {
            red: 80,
            green: 80,
            blue: 80,
        },
    ];
    flat.record_frame(&frame, None, tuning);
    flat.record_frame(&frame, Some(&frame), tuning);
    assert_eq!(flat.recommendation(), "low_temporal_variation");
    assert!(flat.warnings().contains("low_temporal_variation"));

    let mut usable = CaptureQualityStats::default();
    let first = vec![
        RgbAverage {
            red: 160,
            green: 20,
            blue: 20,
        },
        RgbAverage {
            red: 20,
            green: 20,
            blue: 160,
        },
    ];
    let second = vec![
        RgbAverage {
            red: 20,
            green: 160,
            blue: 20,
        },
        RgbAverage {
            red: 160,
            green: 20,
            blue: 120,
        },
    ];
    usable.record_frame(&first, None, tuning);
    usable.record_frame(&second, Some(&first), tuning);
    assert_eq!(usable.recommendation(), "usable");
    assert_eq!(usable.warnings(), "none");
}

#[test]
fn capture_quality_warnings_include_secondary_issues() {
    let tuning = ColorTuning::from(ColorProfile::Vivid);
    let mut stats = CaptureQualityStats::default();
    let frame = vec![
        RgbAverage {
            red: 39,
            green: 39,
            blue: 39,
        },
        RgbAverage {
            red: 42,
            green: 42,
            blue: 41,
        },
    ];
    stats.record_frame(&frame, None, tuning);
    stats.record_frame(&frame, Some(&frame), tuning);

    assert_eq!(stats.recommendation(), "low_temporal_variation");
    let warnings = stats.warnings();
    assert!(warnings.contains("low_luma"));
    assert!(warnings.contains("low_saturation"));
    assert!(warnings.contains("low_temporal_variation"));
}

#[test]
fn capture_quality_hints_match_recommendations() {
    assert_eq!(
        capture_quality_hint("single_channel_area"),
        "choose_multi_channel_area_for_correlation_test"
    );
    assert_eq!(
        capture_quality_hint("low_spatial_separation"),
        "use_region_sampling_or_adjust_channel_regions"
    );
    assert_eq!(
        capture_quality_hint("usable"),
        "capture_is_usable_tune_color_profile_if_needed"
    );
}

#[test]
fn sorts_channels_by_horizontal_position() {
    let area = EntertainmentArea {
        id: "area".into(),
        name: "TV".into(),
        channels: vec![
            EntertainmentChannel {
                channel_id: 8,
                position: Some(EntertainmentChannelPosition {
                    x: 0.8,
                    y: 1.0,
                    z: 0.0,
                }),
            },
            EntertainmentChannel {
                channel_id: 3,
                position: Some(EntertainmentChannelPosition {
                    x: -0.7,
                    y: 1.0,
                    z: 0.0,
                }),
            },
        ],
        lights: None,
    };

    let samples =
        channel_samples_by_position(&area, DEFAULT_SAMPLE_EDGE_MARGIN, SampleCrop::default());
    assert_eq!(
        samples
            .iter()
            .map(|sample| sample.channel_id)
            .collect::<Vec<_>>(),
        vec![3, 8]
    );
    assert_sample_point_close(samples[0].point, 0.08, 0.5);
    assert_sample_point_close(samples[1].point, 0.92, 0.5);
}

#[test]
fn maps_vertical_position_when_channels_have_vertical_span() {
    let area = EntertainmentArea {
        id: "area".into(),
        name: "Stacked".into(),
        channels: vec![
            EntertainmentChannel {
                channel_id: 1,
                position: Some(EntertainmentChannelPosition {
                    x: 0.0,
                    y: -1.0,
                    z: 0.0,
                }),
            },
            EntertainmentChannel {
                channel_id: 2,
                position: Some(EntertainmentChannelPosition {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                }),
            },
        ],
        lights: None,
    };

    let samples =
        channel_samples_by_position(&area, DEFAULT_SAMPLE_EDGE_MARGIN, SampleCrop::default());
    assert_sample_point_close(samples[0].point, 0.5, 0.92);
    assert_sample_point_close(samples[1].point, 0.5, 0.08);
}

#[test]
fn keeps_channels_without_position_after_positioned_channels() {
    let area = EntertainmentArea {
        id: "area".into(),
        name: "Mixed".into(),
        channels: vec![
            EntertainmentChannel {
                channel_id: 8,
                position: None,
            },
            EntertainmentChannel {
                channel_id: 3,
                position: Some(EntertainmentChannelPosition {
                    x: -0.7,
                    y: 1.0,
                    z: 0.0,
                }),
            },
            EntertainmentChannel {
                channel_id: 2,
                position: None,
            },
        ],
        lights: None,
    };

    let samples =
        channel_samples_by_position(&area, DEFAULT_SAMPLE_EDGE_MARGIN, SampleCrop::default());
    assert_eq!(
        samples
            .iter()
            .map(|sample| sample.channel_id)
            .collect::<Vec<_>>(),
        vec![3, 2, 8]
    );
    assert_eq!(samples[1].point, SamplePoint::new(0.5, 0.5));
    assert_sample_point_close(samples[2].point, 0.92, 0.5);
}

#[test]
fn sample_edge_margin_is_configurable() {
    let area = EntertainmentArea {
        id: "area".into(),
        name: "Wide".into(),
        channels: vec![
            EntertainmentChannel {
                channel_id: 1,
                position: Some(EntertainmentChannelPosition {
                    x: -1.0,
                    y: 0.0,
                    z: 0.0,
                }),
            },
            EntertainmentChannel {
                channel_id: 2,
                position: Some(EntertainmentChannelPosition {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                }),
            },
        ],
        lights: None,
    };

    let samples = channel_samples_by_position(&area, 0.2, SampleCrop::default());
    assert_sample_point_close(samples[0].point, 0.2, 0.5);
    assert_sample_point_close(samples[1].point, 0.8, 0.5);
}

#[test]
fn sample_crop_constrains_points_to_content_region() {
    let area = EntertainmentArea {
        id: "area".into(),
        name: "Cropped".into(),
        channels: vec![
            EntertainmentChannel {
                channel_id: 1,
                position: Some(EntertainmentChannelPosition {
                    x: -1.0,
                    y: 1.0,
                    z: 0.0,
                }),
            },
            EntertainmentChannel {
                channel_id: 2,
                position: Some(EntertainmentChannelPosition {
                    x: 1.0,
                    y: -1.0,
                    z: 0.0,
                }),
            },
        ],
        lights: None,
    };

    let crop = SampleCrop::new(0.1, 0.2, 0.25, 0.15).unwrap();
    let samples = channel_samples_by_position(&area, 0.0, crop);
    assert_sample_point_close(samples[0].point, 0.1, 0.25);
    assert_sample_point_close(samples[1].point, 0.8, 0.85);
}

#[test]
fn sample_crop_rejects_invalid_bounds() {
    assert!(SampleCrop::new(-0.1, 0.0, 0.0, 0.0).is_err());
    assert!(SampleCrop::new(0.6, 0.4, 0.0, 0.0).is_err());
    assert!(SampleCrop::new(0.0, 0.0, 0.5, 0.5).is_err());
}

#[test]
fn max_detected_crop_keeps_largest_edge_values() {
    let crop = max_detected_crop(
        DetectedSampleCrop {
            left: 0.1,
            right: 0.3,
            top: 0.0,
            bottom: 0.2,
        },
        DetectedSampleCrop {
            left: 0.2,
            right: 0.1,
            top: 0.4,
            bottom: 0.1,
        },
    );

    assert_eq!(
        crop,
        DetectedSampleCrop {
            left: 0.2,
            right: 0.3,
            top: 0.4,
            bottom: 0.2,
        }
    );
}

#[test]
fn sync_crop_args_renders_copyable_sync_flags() {
    let rendered = sync_crop_args(DetectedSampleCrop {
        left: 0.1,
        right: 0.2,
        top: 0.03,
        bottom: 0.0,
    });

    assert_eq!(
        rendered,
        "crop_args --sample-crop-left 0.1000 --sample-crop-right 0.2000 --sample-crop-top 0.0300 --sample-crop-bottom 0.0000"
    );
}

#[test]
fn cap_detected_crop_limits_auto_crop_per_edge() {
    let crop = cap_detected_crop(
        DetectedSampleCrop {
            left: 0.1,
            right: 0.7,
            top: 0.6,
            bottom: 0.0,
        },
        0.35,
    );

    assert_eq!(
        crop,
        DetectedSampleCrop {
            left: 0.1,
            right: 0.35,
            top: 0.35,
            bottom: 0.0,
        }
    );
}

#[test]
fn validate_auto_crop_max_edge_rejects_invalid_values() {
    assert!(validate_auto_crop_max_edge(-0.1).is_err());
    assert!(validate_auto_crop_max_edge(0.5).is_err());
    assert!(validate_auto_crop_max_edge(0.49).is_ok());
}

#[test]
fn sample_crop_can_merge_detected_crop() {
    let crop = SampleCrop::new(0.1, 0.0, 0.2, 0.0)
        .unwrap()
        .max_detected(DetectedSampleCrop {
            left: 0.05,
            right: 0.3,
            top: 0.4,
            bottom: 0.1,
        })
        .unwrap();

    assert_sample_crop_close(crop, 0.1, 0.3, 0.4, 0.1);
}

#[test]
fn sample_crop_rejects_detected_crop_that_closes_axis() {
    let result = SampleCrop::default().max_detected(DetectedSampleCrop {
        left: 0.5,
        right: 0.5,
        top: 0.0,
        bottom: 0.0,
    });

    assert!(result.is_err());
}

#[test]
fn stage_stats_tracks_average_and_max_milliseconds() {
    let mut stats = StageStats::default();
    stats.record(Duration::from_millis(2));
    stats.record(Duration::from_millis(6));

    assert_eq!(stats.average_ms(), 4.0);
    assert_eq!(stats.max_ms(), 6.0);
}

#[test]
fn frame_delay_uses_sub_millisecond_precision() {
    assert_eq!(frame_delay_for_fps(50), Duration::from_millis(20));
    assert_eq!(frame_delay_for_fps(60), Duration::from_micros(16_666));
}

#[test]
fn validate_fps_rejects_zero() {
    assert!(validate_fps("fps", 0).is_err());
    assert!(validate_fps("fps", 1).is_ok());
}

#[test]
fn validate_capture_poll_ms_rejects_invalid_values() {
    assert!(validate_capture_poll_ms(0).is_err());
    assert!(validate_capture_poll_ms(101).is_err());
    assert!(validate_capture_poll_ms(5).is_ok());
}

#[test]
fn validate_brightness_rejects_invalid_values() {
    assert!(validate_brightness(-0.1).is_err());
    assert!(validate_brightness(1.1).is_err());
    assert!(validate_brightness(0.0).is_ok());
    assert!(validate_brightness(1.0).is_ok());
}

#[test]
fn brightness_scales_average_color() {
    let color = hue_color_from_average(
        RgbAverage {
            red: 200,
            green: 100,
            blue: 50,
        },
        0.5,
        ColorTuning {
            gain: 1.0,
            gamma: 1.0,
            saturation: 1.0,
        },
    );

    assert_eq!(color.red, 100);
    assert_eq!(color.green, 50);
    assert_eq!(color.blue, 25);
}

#[test]
fn vivid_tuning_lifts_dim_capture_and_increases_saturation() {
    let color = hue_color_from_average(
        RgbAverage {
            red: 34,
            green: 28,
            blue: 70,
        },
        1.0,
        ColorTuning::from(ColorProfile::Vivid),
    );

    assert!(color.red > 80);
    assert!(color.green > 60);
    assert!(color.blue > 140);
    assert!(color.blue.abs_diff(color.green) > 60);
}

#[test]
fn boosted_tuning_is_stronger_than_game_for_low_saturation_capture() {
    let average = RgbAverage {
        red: 116,
        green: 108,
        blue: 104,
    };
    let game = hue_color_from_average(average, 1.0, ColorTuning::from(ColorProfile::Game));
    let boosted = hue_color_from_average(average, 1.0, ColorTuning::from(ColorProfile::Boosted));

    assert!(hue_saturation(boosted) > hue_saturation(game));
    assert!(hue_luma(boosted) >= hue_luma(game));
}

#[test]
fn effective_pipewire_fps_defaults_to_highest_cadence() {
    assert_eq!(effective_pipewire_fps(None, 10, 25).unwrap(), 25);
    assert_eq!(effective_pipewire_fps(None, 30, 25).unwrap(), 30);
    assert_eq!(effective_pipewire_fps(Some(60), 10, 25).unwrap(), 60);
    assert!(effective_pipewire_fps(Some(0), 10, 25).is_err());
}

#[test]
fn video_wayland_preset_matches_validated_real_hue_settings() {
    let preset = SyncPresetConfig::from(SyncPreset::VideoWayland);
    assert_eq!(preset.capture_fps, 8);
    assert_eq!(preset.stream_fps, 25);
    assert_eq!(preset.pipewire_fps, 25);
    assert!(matches!(preset.capture_backend, CaptureBackend::Cpu));
    assert_eq!(preset.capture_poll_ms, 5);
    assert!(matches!(preset.sampling, SamplingMode::Region));
    assert!(!preset.auto_crop);
    assert_eq!(preset.max_step, None);
}

#[test]
fn tv_wayland_stays_alias_for_video_wayland() {
    let video = SyncPresetConfig::from(SyncPreset::VideoWayland);
    let legacy = SyncPresetConfig::from(SyncPreset::TvWayland);
    assert_eq!(legacy.capture_fps, video.capture_fps);
    assert_eq!(legacy.stream_fps, video.stream_fps);
    assert_eq!(legacy.pipewire_fps, video.pipewire_fps);
    assert!(matches!(legacy.capture_backend, CaptureBackend::Cpu));
    assert!(matches!(legacy.sampling, SamplingMode::Region));
    assert_eq!(legacy.max_step, video.max_step);
}

#[test]
fn mode_presets_have_distinct_capture_cadence() {
    let video = SyncPresetConfig::from(SyncPreset::VideoWayland);
    let game = SyncPresetConfig::from(SyncPreset::GameWayland);
    let desktop = SyncPresetConfig::from(SyncPreset::DesktopWayland);
    assert_eq!(video.capture_fps, 8);
    assert_eq!(game.capture_fps, 12);
    assert_eq!(desktop.capture_fps, 6);
}

#[test]
fn parses_capture_poll_ms_list() {
    assert_eq!(
        parse_capture_poll_ms_list("5, 8,20").unwrap(),
        vec![5, 8, 20]
    );
    assert!(parse_capture_poll_ms_list("").is_err());
    assert!(parse_capture_poll_ms_list("0").is_err());
}

#[test]
fn synthetic_bench_area_uses_valid_stream_id() {
    let area = synthetic_bench_area();
    assert_eq!(area.id.len(), 36);
    assert_eq!(area.channels.len(), 6);
}

#[test]
fn effective_capture_poll_timeout_keeps_half_stream_budget() {
    assert_eq!(
        effective_capture_poll_timeout(Duration::from_millis(8), Duration::from_millis(40)),
        Duration::from_millis(20)
    );
    assert_eq!(
        effective_capture_poll_timeout(Duration::from_millis(25), Duration::from_millis(40)),
        Duration::from_millis(25)
    );
}

#[test]
fn expected_frames_uses_whole_duration_frames() {
    assert_eq!(expected_frames(10_000, 10), 100);
    assert_eq!(expected_frames(5_500, 25), 137);
}

fn assert_sample_point_close(point: SamplePoint, x: f64, y: f64) {
    assert!((point.x - x).abs() < 0.0001);
    assert!((point.y - y).abs() < 0.0001);
}

fn assert_sample_crop_close(crop: SampleCrop, left: f64, right: f64, top: f64, bottom: f64) {
    assert!((crop.left - left).abs() < 0.0001);
    assert!((crop.right - right).abs() < 0.0001);
    assert!((crop.top - top).abs() < 0.0001);
    assert!((crop.bottom - bottom).abs() < 0.0001);
}

#[test]
fn sync_stats_display_includes_stage_timings() {
    let mut stats = SyncStats {
        capture_backend: Some(CaptureBackend::Gl),
        frames: 2,
        capture_frames: 1,
        repeated_frames: 1,
        missed_capture_frames: 0,
        empty_capture_polls: 3,
        ..SyncStats::default()
    };
    stats.capture.record(Duration::from_millis(4));
    stats.color.record(Duration::from_millis(1));
    stats.encode.record(Duration::from_millis(2));
    stats.send.record(Duration::from_millis(3));

    let rendered = stats.to_string();

    assert!(rendered.contains("sync_stats capture_backend=gl interrupted=false frames=2"));
    assert!(rendered.contains("capture_frames=1"));
    assert!(rendered.contains("repeated_frames=1"));
    assert!(rendered.contains("missed_capture_frames=0"));
    assert!(rendered.contains("empty_capture_polls=3"));
    assert!(rendered.contains("capture_avg_ms=4.000"));
    assert!(rendered.contains("send_max_ms=3.000"));
}
