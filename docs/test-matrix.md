# Linux Wayland Test Matrix

Scénarios produit (modes, musique, tray) : [plan-hue-sync-daily.md](plan-hue-sync-daily.md) — sections Phase 1, 3.5 et Phase 4.

## Target Environments

- GNOME Wayland.
- KDE Wayland.
- Sway / wlroots.
- Hyprland.
- Flatpak sandbox.

## Scenarios

- Fixed-color DTLS stream on a real lighting zone.
- Portal-selected Wayland stream capture.
- End-to-end portal average color sync to the lighting stream.
- First launch.
- Portal capture permission accepted.
- Portal capture permission refused.
- Capture stream closed while syncing.
- Resume after sleep.
- Monitor hotplug.
- Multi-monitor selection.
- Lighting controller unreachable.
- Lighting controller lost during sync.
- No lighting zone configured.
- DTLS handshake failure.
- Missing GStreamer plugin.
- Flatpak execution.
- Non-Flatpak development execution.

## Planned scenarios (plan-hue-sync-daily)

- Sync mode **Video** / **Game** / **Desktop** preset applied from GUI.
- Sync mode **Music** via `lumaway audio-sync` (no Portal capture) when Phase 3 / v1.1 is in scope.
- Phase 3 / v1.1 Music: default vs explicit PipeWire/Pulse audio source.
- Phase 3 / v1.1 Music: audio source unavailable or changed while syncing.
- Phase 3 / v1.1 Music: long silence → dimmed or off lights within 5 s.
- StatusNotifier/AppIndicator Start/Stop when the desktop exposes a tray; on GNOME without tray support, validate the fallback window Start/Stop flow plus minimal critical-error notification when available.
- First-run wizard: discover bridge → pair → load area → test color → start sync.
- Non-developer UX checklist ([plan-hue-sync-daily.md](plan-hue-sync-daily.md) §3.4): no terminal, no app-key on home screen.
- i18n: `LANG=fr_FR.UTF-8` — home screen fully French; unknown locale falls back to English (§3.5).
- i18n: `LANG=de_DE.UTF-8` — when `de.po` ships, primary UI in German.
- Install script installs compiled `.mo` under `~/.local/share/locale/`.
- Install script installs translated `.desktop` and AppStream `.metainfo.xml`.
- Robustness P0 ([plan-hue-sync-daily.md](plan-hue-sync-daily.md) §15.3): resume after sleep; bridge lost during sync; Portal stream closed; DTLS failure recovery; entertainment area conflict message.
- No entertainment area configured: guided flow to create zone in Hue app.
- Single GUI instance: second launch focuses existing window.
- GUI classified errors: Portal/capture/bridge failures expose contextual `Retry` and/or `Open Settings` recovery actions.
- About dialog: version, MPL-2.0, local processing, no telemetry, project links, and nominative Philips Hue compatibility scope.
- No Entertainment zone: empty `list-areas` result disables the zone switch, shows a guided no-zone state, and explains how to create a Hue Entertainment area.
- Portal flow: GUI shows the translated screen/window selection reminder; `lumaway sync` reuses and persists `LUMAWAY_PORTAL_RESTORE_TOKEN` when the portal provides one.
- Portal stream closed: after more than 5 seconds without new frames, sync exits with a classified Portal-stream error and the GUI exposes `Retry`.
- Bridge lost during sync: a mid-stream DTLS send failure is annotated as bridge loss, stops the sync loop, and maps to a translated GUI recovery message.
- Resume after sleep: a wall-clock / monotonic-clock gap greater than 5 seconds stops the sync loop and maps to a translated GUI recovery message.
- `tv-wayland` preset still works as alias after `video-wayland` is introduced.

## Success Criteria

- Error is explained.
- No local secret appears in logs.
- The engine stops cleanly.
- CLI returns a meaningful exit code.
- GUI remains responsive once it exists.
- `lumaway doctor` identifies the likely cause once implemented.

## Verified Smoke Tests

- `cargo test`: all workspace unit and doc tests pass.
- `lumaway test-color`: real controller `192.168.1.108`, area `TV`, fixed RGB stream validated.
- `/home/bunny/.local/bin/lumaway test-color --color red --duration-ms 1200 --fps 25`: real quick zone test completed with 30 stream frames.
- `lumaway capture-stats --portal --duration-ms 2000`: real GNOME Wayland portal stream validated.
- `lumaway discover-bridges`: real controller discovery returned `192.168.1.108` through local fallback discovery.
- `lumaway sync --bridge 192.168.1.108 --area TV --duration-ms 5000 --fps 10`: first portal average color sync completed and sent 16 frames.
- `lumaway sync --bridge 192.168.1.108 --area TV --duration-ms 5000 --fps 10 --smoothing 0.35 --noise-threshold 3 --max-step 32`: spatial sync completed and sent 37 frames; `sync_stats` baseline captured.
- `lumaway sync --bridge 192.168.1.108 --area TV --duration-ms 5000 --fps 10 --sample-width 160 --sample-height 90 --smoothing 0.35 --noise-threshold 3 --max-step 32`: CPU-sampled spatial sync completed and sent 41 frames; capture avg improved to 20.943 ms.
- `lumaway sample-bench --portal --frames 20 --bands 2 --grids 80x45,120x68,160x90,240x135`: Portal benchmark completed; `120x68` selected as the default grid.
- `lumaway list-areas --bridge 192.168.1.108`: real channel positions are parsed and exposed.
- `lumaway sync --bridge 192.168.1.108 --area TV --duration-ms 5000 --fps 10 --sample-width 120 --sample-height 68 --smoothing 0.35 --noise-threshold 3 --max-step 32`: 2D point-sampled sync completed and sent 48 frames; capture avg measured at 3.032 ms.
- `lumaway detect-crop --portal --frames 5 --sample-width 120 --sample-height 68 --fps 10 --threshold 8`: real GNOME Wayland stream returned stable `crop_suggested` top crop `0.0147`.
- `lumaway sync-bench --duration-ms 10000 --capture-fps 8 --stream-fps 25 --pipewire-fps 25 --capture-backend cpu --capture-poll-ms 5`: real GNOME Wayland portal benchmark completed with 250 stream frames, 92 accepted capture frames, and 0 missed target captures.
- `lumaway sync-bench --duration-ms 10000 --capture-fps 8 --stream-fps 25 --pipewire-fps 25 --capture-backend gl --capture-poll-ms 5`: real GNOME Wayland portal benchmark completed with 250 stream frames, 143 accepted capture frames, and 0 missed target captures.
- `lumaway sync --bridge 192.168.1.108 --area TV --duration-ms 10000 --preset tv-wayland`: real sync with GL capture completed with 250 stream frames, 129 accepted capture frames, 0 missed target captures, send max 0.040 ms.
- `LUMAWAY_BRIDGE=192.168.1.108 LUMAWAY_AREA=TV LUMAWAY_APP_KEY=... LUMAWAY_CLIENT_KEY=... lumaway sync --preset tv-wayland --duration-ms 5000`: daily environment-driven real sync completed with `capture_backend=gl`, 125 stream frames, 74 accepted capture frames, 0 missed target captures, send max 0.024 ms.
- `LUMAWAY_BRIDGE=192.168.1.108 LUMAWAY_AREA=TV LUMAWAY_APP_KEY=... LUMAWAY_CLIENT_KEY=... lumaway sync --preset tv-wayland --duration-ms 0`: real long-running sync stopped with Ctrl-C; shutdown returned exit code 0, reported `interrupted=true`, sent 263 frames, accepted 143 capture frames, had 0 missed target captures, and left no lingering `lumaway` process.
- `scripts/install-desktop-app.sh`: shell syntax validated; installer completed as user `bunny`, installed `~/.local/bin/lumaway`, `~/.local/bin/lumaway-gui`, and `~/.local/share/applications/io.github.BunnySweety.LumaWay.desktop`, while keeping `~/.config/lumaway/lumaway.env` at mode `0600`.
- `cargo check -p lumaway-gui`: GTK/libadwaita application crate compiles.
- `cargo test -p lumaway-gui`: GUI parsing for auth JSON and zone JSON is covered.
- `cargo test -p lumaway-gui`: XDG session autostart desktop-entry rendering and Exec path quoting are covered.
- `cargo test -p lumaway-gui`: classified GUI errors expose the expected `Retry` / `Open Settings` action policy.
- `cargo test --workspace`: Portal restore-token normalization and GUI Portal status derivation are covered.
- `cargo test --workspace`: stale Portal capture stream detection and GUI classification are covered.
- `/home/bunny/.local/bin/lumaway-gui`: real GTK/libadwaita application opens in the GNOME Wayland session and remains running.
- `gio launch ~/.local/share/applications/io.github.BunnySweety.LumaWay.desktop`: desktop entry starts the GTK/libadwaita application in the GNOME Wayland session.
- `/home/bunny/.local/bin/lumaway list-areas --bridge 192.168.1.108`: the real controller returned three zones, including `TV`; this validates the data path used by the GUI zone-loading action.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: real GUI Start path selected the Portal stream, used `capture_backend=gl`, completed with 125 frames, accepted 125 capture frames, had 0 missed target captures, and exited with code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after the brand-neutral UX pass selected the Portal stream, used `capture_backend=gl`, completed with 125 frames, accepted 125 capture frames, had 0 missed target captures, and exited with code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after automatic zone loading completed with `capture_backend=gl`, 125 frames, 125 accepted capture frames, 0 missed target captures, and exit code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after discovery and health-status GUI pass completed with `capture_backend=gl`, 125 frames, 0 missed target captures, and exit code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after quick zone-test GUI pass completed with `capture_backend=gl`, 125 frames, 0 missed target captures, and exit code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after intensity/reactivity sliders passed `brightness=1.0` and `smoothing=0.35`, completed with `capture_backend=gl`, 125 frames, 0 missed target captures, and exit code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after capture-status parsing saw Portal selection and `effective_capture_backend=Gl`, completed with 125 frames, 0 missed target captures, and exit code 0.
- `LUMAWAY_GUI_AUTOSTART=1 LUMAWAY_GUI_QUIT_AFTER_SYNC=1 LUMAWAY_GUI_ECHO_LOGS=1 LUMAWAY_GUI_DURATION_MS=5000 /home/bunny/.local/bin/lumaway-gui`: rerun after adding the persistent app-open autostart toggle completed with `capture_backend=gl`, 125 frames, 0 missed target captures, and exit code 0; local `LUMAWAY_AUTOSTART_SYNC` was reset to `false`.
- `/home/bunny/.local/bin/lumaway-gui`: redesigned dark visual shell starts in the GNOME Wayland session without GTK CSS warnings; an autostart smoke attempt was stopped manually when the Portal selector did not return a stream.
- `/home/bunny/.local/bin/lumaway-gui`: main window starts with a visible `Settings` header button; short launch smoke completed without GTK/CSS warnings, then was terminated by timeout with no lingering process.
- `/home/bunny/.local/bin/lumaway sample-debug --portal --preset tv-wayland --frames 1 --color-profile desktop`: real GNOME Wayland Portal capture completed without Hue streaming; printed per-channel raw/smoothed/graded/output RGB, luma, saturation, sample point, sample radius, and capture timing. Example validation: raw `25,39,53` graded to `50,77,102` with output luma `73.1`.
- `/home/bunny/.local/bin/lumaway sample-debug --portal --preset tv-wayland --area TV --frames 1 --sampling region --color-profile desktop`: real GNOME Wayland Portal capture completed without Hue streaming; printed `sampling=Region`, CPU backend, TV left/right weighted regions, and per-channel graded output RGB.
- `/home/bunny/.local/bin/lumaway backend-probe --frames 5 --sample-width 120 --sample-height 68 --fps 25`: real GNOME Wayland Portal probe confirmed CPU usable (`max_rgb=131`, `avg_luma=113.4`) while GL started but returned black (`max_rgb=0`, `avg_luma=0.0`); recommendation was `backend=cpu`.
- `/home/bunny/.local/bin/lumaway capture-quality --portal --preset tv-wayland --frames 10`: command is installed and now loads `~/.config/lumaway/lumaway.env` automatically; live capture did not proceed because the current saved Hue application key is rejected by the bridge, and `doctor` also reports Portal session warnings outside the graphical user session.
- `/home/bunny/.local/bin/lumaway doctor`: Hue authentication failures now include a repair hint to press the bridge button and pair again from Settings, instead of only reporting the raw `Hue bridge authentication failed` error.
- `cargo test`: GUI error classification covers Hue authentication failures and keeps unrelated Portal errors out of the pairing-required path.
- `/home/bunny/.local/bin/lumaway-gui`: Pairing success now writes refreshed Hue keys to `~/.config/lumaway/lumaway.env` immediately before attempting to load zones, so a later zone-loading error does not lose the repaired credentials.
- `/home/bunny/.local/bin/lumaway-gui`: Settings now exposes a `Quality` action that runs the installed `capture-quality` diagnostic and prints a compact recommendation, recommended action, backend, luma, saturation, frame-delta, channel-separation, and dark-frame summary in the app log.
- `Quality` on a one-light area is classified as `single_channel_area`; use the `TV` area or another multi-light entertainment area when validating correlation between lights.
- `Quality` reports secondary warnings such as `low_luma`, `low_saturation`, and `low_temporal_variation` together, so a stable measurement does not hide weak brightness or weak colorfulness.
- `cargo test`: `boosted` color profile is covered as stronger than `game` on low-saturation captures and is accepted by the GUI profile sanitizer.
- `/home/bunny/.local/bin/lumaway sample-debug --portal --area TV --frames 1 --capture-backend auto --sampling region --color-profile desktop`: real GNOME Wayland Portal capture detected dark GL output during the auto quality probe, logged fallback to CPU, and completed with `capture_backend=cpu`, `sampling=Region`, and non-black per-channel output.
- `LUMAWAY_PROFILE=live /home/bunny/.local/bin/lumaway sample-debug --portal --area TV --frames 1`: real GNOME Wayland Portal capture loaded non-secret profile defaults from `~/.config/lumaway/profiles/live.env` equivalent test config, applied `preset=tv-wayland`, `capture_backend=auto`, `sampling=region`, `color_profile=desktop`, detected dark GL, and completed on CPU.
- `XDG_CONFIG_HOME=<tmp> /home/bunny/.local/bin/lumaway profile-template --name testprofile`: created a starter non-secret profile containing capture backend, cadence, sampling, brightness, reactivity, color profile, and noise-threshold defaults.
- `XDG_CONFIG_HOME=<tmp> /home/bunny/.local/bin/lumaway profile-list`: listed sorted `.env` profiles and ignored non-profile files.
- `cargo test`: CLI config loading is covered so `~/.config/lumaway/lumaway.env` supplies bridge/app/profile defaults without overriding explicit shell variables.
- `XDG_CONFIG_HOME=<tmp> /home/bunny/.local/bin/lumaway calibrate-capture --name tvtest --frames 3 --force`: real GNOME Wayland Portal calibration wrote a measured profile with `LUMAWAY_CAPTURE_BACKEND=cpu` after CPU returned usable frames and GL returned black frames.
- `/home/bunny/.local/bin/lumaway-gui`: real GTK/libadwaita launch smoke completed after adding the Settings `Capture profile` field and profile propagation to sync. Local autostart began a long-running sync during the smoke; the spawned sync was stopped and the TV area was deactivated afterward.
- `cargo test`: weighted rectangular `SampleRegion` averaging is covered with a synthetic RGB frame; the `tv-wayland` preset is covered as CPU capture plus region sampling.
- `cargo test`: profile path validation, non-secret profile key allowlist, calibrated profile generation, and GUI profile-name sanitization are covered.
- `LUMAWAY_PROFILE=diag lumaway doctor`: profile diagnostics report `profile.file` ok for supported profile keys and `profile.ignored_keys` warning for unsupported keys.
- `LUMAWAY_PROFILE=missing lumaway doctor`: profile diagnostics report `profile.load` as an error instead of exiting before diagnostics; other commands still fail clearly when the selected profile is missing.
- `cargo test`: temporal smoothing behavior is covered with unit tests.
- `cargo test`: anti-noise threshold behavior is covered with unit tests.
- `cargo test`: per-frame max-step limiting is covered with unit tests.
- `cargo test`: sync timing metric aggregation and display are covered with unit tests.
- `cargo test`: reusable stream-frame encoding is covered with unit tests.
- `cargo test`: sync cadence helpers, capture poll validation, and extended sync stats counters are covered with unit tests.
- `cargo test`: mid-sync DTLS send failures are annotated as bridge loss and GUI classification maps them to the bridge-lost recovery message.
- `cargo test`: sleep-resume gap detection and GUI classification map resumed sessions to the sleep recovery message.
- `cargo test`: About dialog copy covers local processing, no telemetry, Philips Hue compatibility scope, and non-affiliation.
- `cargo test`: empty area-list selection keeps no zone selected instead of picking a stale value.
- `cargo test`: relative 2D channel sample mapping, configurable edge margin, manual crop bounds, same-height vertical centering, and fallback placement are covered with unit tests.
- `cargo test`: sampled dark-border detection, fully dark frame handling, crop aggregation, auto-crop edge cap validation, manual-plus-auto crop merging, and copyable crop args are covered with unit tests.

## Current Gaps

Product roadmap items (music, tray, mode selector) are tracked in [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

- Portal scaled RGB conversion path must be remeasured after accepting the GNOME Portal selector.
- Sync uses 2D point or weighted-region sampling, not full 3D-aware placement.
- Smoothing, noise-threshold, and max-step defaults need visual tuning on real content.
- `--sample-edge-margin` and manual crop defaults need visual tuning on real content and black-bar scenarios.
- `--auto-crop` still needs real Portal validation on black-bar video content.
- GStreamer-side Portal scaling needs a safer implementation; native sample caps are used for now.
- CPU sampling grid defaults to `120x68`; quality still needs tuning across content types.
- No automated integration test exists for local lighting hardware or Portal permission flows.
