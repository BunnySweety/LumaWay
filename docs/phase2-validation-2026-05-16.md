# Phase 2 Validation Evidence - 2026-05-16

Status: code/docs delivered; field release proof still requires external video, no-silent-black observation, and a timed observer run.

## Scope

This file records the current Phase 2 evidence against `docs/plan-hue-sync-daily.md`.

Repository state:

- implementation commit: `593bd0a` (`Complete Phase 2 screen quality tasks`);
- TV validation commit: `5bdb192` (`Record Phase 2 TV validation`);
- field-gate clarification commit: `7e8286e` (`Clarify Phase 2 field validation gates`);
- validation audit commit: `ab8725d` (`Add Phase 2 validation audit`);
- latency helper commit: `7533b08` (`Add Phase 2 latency helper`);
- first-run helper commit: `48093b5` (`Add Phase 2 first-run helper`);
- CI helper coverage commit: `b914fef` (`Cover Phase 2 validation helpers in CI`);
- documentation status commit: `2c93ad1` (`Refresh Phase 2 documentation status`);
- next-action alignment commit: `5b36a06` (`Update Phase 2 next actions`);
- field evidence helper commit: `c3d10e4` (`Add Phase 2 field evidence helper`);
- silent-black evidence gate commit: `b74ff6c` (`Track Phase 2 silent black evidence`);
- required silent-black flag coverage commit: `7a6db70` (`Cover required silent black evidence flag`);
- plan latency wording alignment commit: `936e75b` (`Align Phase 2 latency threshold wording`);
- field preflight helper commit: `a17b966` (`Add Phase 2 field preflight helper`);
- CI evidence for the automated Phase 2 artifacts: `25973281682` on `a17b966`, workflow `CI`, job `rust`, conclusion `success`.

## Completion Audit

Objective: deliver every Phase 2 screen-quality item for daily Hue screen sync.

Concrete success criteria from the plan and linked Phase 2 docs:

- 2.1: persistent manual crop profile values are supported by daily sync and diagnostics.
- 2.2: Video / `vivid` avoids false black output while preserving true black.
- 2.3: black/too-dark capture failures guide the user toward backend probing.
- 2.4: fixed comparison harness exists, including latency evidence fields and a 300 ms visible-reaction gate.
- 2.5: diagnostics are reusable from CLI/GUI validation flows.
- 2.6: Hue Entertainment `position.z` is used as a vertical fallback when `position.y` has no usable span.
- Phase 2 finish criterion: an existing TV user can reach satisfactory sync without mandatory `calibrate-capture`.
- Phase 2 finish criterion: a new user reaches first satisfactory screen sync in 10 minutes or less.
- Release gate: visible screen-to-light reaction is <= 300 ms for at least 5 accepted full-screen transitions, with no silent black session.

Prompt-to-artifact checklist:

| Requirement / gate | Artifact inspected | Evidence | Audit status |
|--------------------|--------------------|----------|--------------|
| 2.1 persistent crop profile values | `crates/lumaway-cli/src/cli_args.rs`, `crates/lumaway-cli/src/profile_env.rs`, tests | Tests include `sync_reads_sample_crop_from_environment` and `profile_template_includes_persistent_crop_defaults`; selected profile crop keys are non-secret profile values used by `sync`, `sample-debug`, and `capture-quality`. | Covered |
| 2.2 Video / `vivid` black handling | `crates/lumaway-cli/src/color_tuning.rs`, tests | Test `vivid_tuning_lifts_dim_non_black_video_without_lighting_black` covers black/noise staying dark and dim non-black content receiving a soft luminance floor. | Covered |
| 2.3 backend-probe assistant | `crates/lumaway-gui/src/main.rs`, `crates/lumaway-gui/src/user_messages.rs`, tests | `CaptureTooDark` exposes `Probe backend`; test `formats_backend_probe_summary` covers the CPU/GL recommendation summary. | Covered |
| 2.4 fixed harness and latency verifier | `docs/phase2-comparison-harness.md`, `docs/fixtures/phase2-patterns.html`, `scripts/phase2-latency-summary.sh`, CI | Harness lists fixed patterns and result fields; helper enforces at least 5 transitions and <= 300 ms; workflow step `phase2 validation helpers` covers pass/fail examples. | Covered except external video evidence |
| 2.5 reusable diagnostics | `docs/phase2-comparison-harness.md`, `docs/test-matrix.md`, CLI/GUI evidence | Harness uses `backend-probe`, `capture-quality`, and `sample-debug`; test matrix records CLI diagnostics plus GUI `Quality` action evidence. | Covered |
| 2.6 `position.z` vertical fallback | `crates/lumaway-cli/src/sampling.rs`, tests | Tests `maps_depth_position_when_vertical_span_is_missing` and `vertical_position_takes_priority_over_depth_position` cover fallback and Y priority. | Covered |
| Existing TV user does not require `calibrate-capture` | Real TV command evidence below | `sync --area TV --duration-ms 3000 --capture-backend auto` started without `calibrate-capture`, selected CPU after GL-black fallback, sent 75 frames, and shut down cleanly. | Covered for this environment |
| New-user first satisfactory sync <= 10 min | `scripts/phase2-first-run-summary.sh`, CI | Helper enforces <= 600 seconds and `--calibrate-used no`; CI covers pass/fail examples. No observed new-user timing run exists. | Missing external observer run |
| Visible reaction <= 300 ms, >= 5 transitions | `scripts/phase2-latency-summary.sh`, CI | Helper enforces the numeric gate from video frame pairs; CI covers pass/fail examples. No camera video evidence exists from this environment. | Missing external video |
| Field capture preflight | `scripts/phase2-field-preflight.sh`, CI | Helper checks local helper/harness files and camera availability before a manual run; CI covers camera-optional pass and missing-camera fail paths. | Covered |
| Combined field evidence block | `scripts/phase2-field-evidence.sh`, CI | Helper wraps latency, first-run, and no-silent-black verifiers into one pasteable audit block and exits non-zero if any gate fails. | Covered except external measurements |
| CI actually covers validators | `.github/workflows/ci.yml`, GitHub Actions run `25973281682` | `cargo fmt`, `cargo clippy`, `cargo test`, and `phase2 validation helpers` all completed with conclusion `success` on the Phase 2 preflight baseline `a17b966`. | Covered |

Completion verdict: Phase 2 is delivered for code, docs, helpers, and available TV validation, but the objective is not fully complete until the external field evidence block is captured and recorded.

## Deliverable Checklist

| Requirement | Evidence | Status |
|-------------|----------|--------|
| 2.1 persistent manual crop profile via `LUMAWAY_SAMPLE_CROP_LEFT/RIGHT/TOP/BOTTOM`, reused by `sync`, `sample-debug`, and `capture-quality` | `crates/lumaway-cli/src/cli_args.rs` reads crop env keys for all three commands; `crates/lumaway-cli/src/profile_env.rs` includes crop defaults and whitelists profile keys; tests `sync_reads_sample_crop_from_environment` and `profile_template_includes_persistent_crop_defaults` pass in CI. | Delivered |
| 2.2 Video / `vivid` preserves true black and near-black noise while lifting dim non-black content | `crates/lumaway-cli/src/color_tuning.rs`; test `vivid_tuning_lifts_dim_non_black_video_without_lighting_black` confirms black/noise stay off and dim content reaches soft output luma. | Delivered |
| 2.3 GUI suggests `backend-probe` after black/too-dark capture | `crates/lumaway-gui/src/main.rs` shows `Probe backend` after `CaptureTooDark`, runs `lumaway backend-probe`, and formats CPU/GL summary; `crates/lumaway-gui/src/user_messages.rs` recovery action points to backend probe; test `formats_backend_probe_summary` passes in CI. | Delivered |
| 2.4 comparison harness with fixed patterns, diagnostics, latency threshold, and result template | `docs/phase2-comparison-harness.md` and `docs/fixtures/phase2-patterns.html` define fixed patterns, `backend-probe`, `capture-quality`, `sample-debug`, internal latency guard, visible latency gate <= 300 ms, no-silent-black field, and result fields; `scripts/phase2-latency-summary.sh` converts observed video frame pairs into pass/fail latency evidence; `scripts/phase2-first-run-summary.sh` converts observed setup time into pass/fail first-run evidence; `scripts/phase2-field-evidence.sh` combines latency, first-run, and no-silent-black gates. | Delivered |
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
- no-silent-black observation: confirm no non-black pattern stayed black silently during the manual run;
- new-user timing: start from installed app, no prior capture calibration, and time until first satisfactory non-black TV/monitor sync; pass threshold is <= 10 minutes.

No `/dev/video*` capture device is available in this environment, so these field criteria cannot be completed by the agent here.

Use `scripts/phase2-field-preflight.sh` on the target machine before recording to confirm that
the helper files and a `/dev/video*` camera are available.

Use `scripts/phase2-latency-summary.sh` after reading video frame numbers to produce a repeatable
pass/fail result for the 300 ms visible-latency gate.

Use `scripts/phase2-first-run-summary.sh` after timing the installed app flow to produce a
repeatable pass/fail result for the 10-minute no-calibration first-run gate.

Use `scripts/phase2-field-evidence.sh` when both measurements are available to produce a
single pasteable evidence block for this audit. Pass `--silent-black no` only when the
manual run confirms that no non-black pattern stayed black silently.

If this validation needs to be tracked as a GitHub issue, use
`.github/ISSUE_TEMPLATE/phase2-field-validation.md`.
