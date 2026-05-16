# Phase 2 Validation Evidence - 2026-05-16

Status: code/docs delivered; field release proof still requires external video and a timed observer run.

## Scope

This file records the current Phase 2 evidence against `docs/plan-hue-sync-daily.md`.

Repository state:

- implementation commit: `593bd0a` (`Complete Phase 2 screen quality tasks`);
- TV validation commit: `5bdb192` (`Record Phase 2 TV validation`);
- field-gate clarification commit: `7e8286e` (`Clarify Phase 2 field validation gates`);
- latest validated CI run: `25948910463` on `7e8286e`, workflow `CI`, job `rust`, conclusion `success`.
- validation audit commit: `ab8725d` (`Add Phase 2 validation audit`).

## Deliverable Checklist

| Requirement | Evidence | Status |
|-------------|----------|--------|
| 2.1 persistent manual crop profile via `LUMAWAY_SAMPLE_CROP_LEFT/RIGHT/TOP/BOTTOM`, reused by `sync`, `sample-debug`, and `capture-quality` | `crates/lumaway-cli/src/cli_args.rs` reads crop env keys for all three commands; `crates/lumaway-cli/src/profile_env.rs` includes crop defaults and whitelists profile keys; tests `sync_reads_sample_crop_from_environment` and `profile_template_includes_persistent_crop_defaults` pass in CI. | Delivered |
| 2.2 Video / `vivid` preserves true black and near-black noise while lifting dim non-black content | `crates/lumaway-cli/src/color_tuning.rs`; test `vivid_tuning_lifts_dim_non_black_video_without_lighting_black` confirms black/noise stay off and dim content reaches soft output luma. | Delivered |
| 2.3 GUI suggests `backend-probe` after black/too-dark capture | `crates/lumaway-gui/src/main.rs` shows `Probe backend` after `CaptureTooDark`, runs `lumaway backend-probe`, and formats CPU/GL summary; `crates/lumaway-gui/src/user_messages.rs` recovery action points to backend probe; test `formats_backend_probe_summary` passes in CI. | Delivered |
| 2.4 comparison harness with fixed patterns, diagnostics, latency threshold, and result template | `docs/phase2-comparison-harness.md` and `docs/fixtures/phase2-patterns.html` define fixed patterns, `backend-probe`, `capture-quality`, `sample-debug`, internal latency guard, visible latency gate <= 300 ms, and result fields; `scripts/phase2-latency-summary.sh` converts observed video frame pairs into pass/fail latency evidence. | Delivered |
| 2.5 diagnostics reuse `sample-debug` and `capture-quality` | `docs/desktop-app.md` documents GUI `Quality`; `docs/phase2-comparison-harness.md` uses both diagnostics; `docs/test-matrix.md` contains live diagnostic evidence. | Delivered |
| 2.6 Entertainment `position.z` handling | `crates/lumaway-cli/src/sampling.rs` uses `position.y` first and falls back to `position.z` when Y has no span; tests `maps_depth_position_when_vertical_span_is_missing` and `vertical_position_takes_priority_over_depth_position` pass in CI. | Delivered |

## Real TV Evidence

Environment:

- desktop/session: GNOME Wayland;
- bridge: `192.168.1.108`;
- zone: `TV` (`d7f38af5-4a85-404a-b555-588ff445f3f3`);
- installed app rebuilt from latest local checkout on 2026-05-16 03:11 local time.

Observed commands:

```text
/tmp/lumaway-codex-target/debug/lumaway doctor
```

Result: portal services active, bridge reachable, 3 Entertainment areas found, application ID available.

```text
/tmp/lumaway-codex-target/debug/lumaway backend-probe --frames 5 --sample-width 120 --sample-height 68 --fps 25
```

Result: CPU usable (`frames=5/5`, `max_rgb=188`, `avg_luma=149.5`), GL black (`max_rgb=0`, `avg_luma=0.0`), recommendation `backend=cpu`.

```text
/tmp/lumaway-codex-target/debug/lumaway sample-debug --portal --sync-mode video --preset video-wayland --area TV --frames 1 --sampling region --color-profile vivid
```

Result: CPU capture, `sampling=Region`, two TV channels sampled, both channels produced non-black output.

```text
/tmp/lumaway-codex-target/debug/lumaway capture-quality --portal --sync-mode video --preset video-wayland --area TV --frames 10 --sampling region --color-profile vivid
```

Result: CPU capture, `dark_frames=0`; remaining warnings were content-related (`low_saturation`, `low_temporal_variation`), not black capture.

```text
/tmp/lumaway-codex-target/debug/lumaway sync --sync-mode video --preset video-wayland --area TV --duration-ms 3000 --sampling region --color-profile vivid --capture-backend auto
```

Result: sync started without `calibrate-capture`; GL black output detected and CPU fallback selected; Hue DTLS handshake completed; 75 sync frames sent; Entertainment deactivated after the requested duration.

Internal latency guard from `sync_stats`:

```text
capture_max_ms=20.069
send_max_ms=4.886
20.069 + 4.886 + 80 = 104.955 ms
```

This is below the 300 ms internal guard, but it does not prove visible light-response latency.

## Release Proof Still Missing

The remaining Phase 2 finish criteria require a person and/or camera:

- visible latency: record screen and Hue lights together at 120 fps or higher and measure at least 5 accepted full-screen black/white transitions, each <= 300 ms;
- new-user timing: start from installed app, no prior capture calibration, and time until first satisfactory non-black TV/monitor sync; pass threshold is <= 10 minutes.

No `/dev/video*` capture device is available in this environment, so these two criteria cannot be completed by the agent here.

Use `scripts/phase2-latency-summary.sh` after reading video frame numbers to produce a repeatable
pass/fail result for the 300 ms visible-latency gate.
