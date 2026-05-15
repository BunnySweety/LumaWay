# LumaWay

LumaWay is a native Linux Wayland ambient-light sync application.

The project targets:

- Linux Wayland;
- XDG Desktop Portal;
- PipeWire / GStreamer;
- a local lighting controller with low-latency streaming support;
- GTK4/libadwaita;
- Flatpak distribution.

The current version has a GTK/libadwaita desktop shell over a validated headless engine: controller authentication, lighting-zone activation, low-latency local streaming, Wayland capture through XDG Desktop Portal, and PipeWire/GStreamer frame sampling.

## Initial Scope

```text
lumaway test-color --bridge <ip> --area <id> --color red
```

The GTK/libadwaita application now wraps the validated streaming and capture/sync pipeline.

## Current CLI

Create local credentials after pressing the controller link button:

```text
lumaway auth --bridge <ip>
```

List configured lighting zones:

```text
lumaway list-areas --bridge <ip> --app-key <app-key>
```

Start the fixed-color spike. This fetches the zone channels, builds RGB stream frames, activates the zone through the local controller API, then attempts a DTLS-PSK connection to stream the fixed color briefly:

```text
lumaway test-color --bridge <ip> --app-key <app-key> --client-key <client-key> --area <id> --color red
```

Optional stream controls:

```text
lumaway test-color --bridge <ip> --area <id> --color red --duration-ms 3000 --fps 25
```

Named colors and hex colors are supported:

```text
--color red
--color '#ff8000'
```

Run initial environment checks:

```text
lumaway doctor
lumaway doctor --bridge <ip> --app-key <app-key>
```

`doctor` also validates `LUMAWAY_PROFILE` when set, including whether the profile file exists and whether it contains unsupported keys.

Optional **Hue HTTPS TLS pinning**: `LUMAWAY_HUE_PIN_CERTS=1` (default hashes leaf **SPKI**; `LUMAWAY_HUE_PIN_MODE=cert` for legacy full-cert pins). **`LUMAWAY_BRIDGE_ID`** is saved automatically after pairing or `bridge-info` and binds pins under `hue-tls-pins/by-id/`. See `docs/security.md`. `doctor` reports pinning mode and pin files.

Discover the local lighting controller:

```text
lumaway discover-bridges
```

`--bridge` and `--area` can also come from `LUMAWAY_BRIDGE` and `LUMAWAY_AREA`. Local credentials can come from `LUMAWAY_APP_KEY` and `LUMAWAY_CLIENT_KEY`.

Run the first GStreamer appsink capture smoke test:

```text
lumaway capture-stats --duration-ms 2000 --width 320 --height 180 --fps 30
```

Run capture stats against a real portal-selected Wayland stream:

```text
lumaway capture-stats --portal --duration-ms 2000
```

This uses XDG Desktop Portal to select a stream, opens the PipeWire remote FD, and connects GStreamer with `pipewiresrc fd=<portal fd> path=<node id>`. `--width`, `--height`, and `--fps` define the RGB appsink profile.

Compare CPU and GL capture quality on the same Portal-selected stream:

```text
lumaway backend-probe --frames 5 --sample-width 120 --sample-height 68 --fps 25
```

The probe prints one row per backend with accepted frames, max RGB, average luma, timing, dark-frame detection, and a conservative backend recommendation. Use it when `--capture-backend gl` looks black or weak, or before changing the daily preset away from CPU.

Run the first end-to-end screen average sync:

```text
lumaway sync --bridge <ip> --app-key <app-key> --client-key <client-key> --area <id> --duration-ms 5000 --fps 25
```

`--fps` remains a compatibility shortcut that sets both capture and lighting stream cadence. Use `--capture-fps` to tune screen sampling separately from `--stream-fps`, which controls how often frames are sent to the controller. The PipeWire pipeline runs at `max(capture-fps, stream-fps)` by default; override that with `--pipewire-fps` when benchmarking Portal behavior. `--capture-backend auto|cpu|gl` selects the capture pipeline; `auto` now probes GL output quality and falls back to CPU if GL starts but returns black/unusable frames, while `gl` forces GStreamer GL upload/color conversion/download before `appsink`. When streaming faster than capture, LumaWay repeats the last computed color frame instead of blocking the lighting stream on screen capture. `--brightness` scales output intensity from `0.0` to `1.0`, and `--smoothing` controls reactivity from slow/smooth to immediate. `--capture-poll-ms` controls how long each non-initial capture poll may wait for a fresh Portal/PipeWire frame; the default is `5`. Set `--duration-ms 0` to run until Ctrl-C; shutdown still deactivates the active lighting zone.

```text
lumaway sync --bridge <ip> --area <id> --capture-fps 8 --stream-fps 25 --pipewire-fps 25 --capture-backend cpu --capture-poll-ms 5
```

The validated GNOME Wayland TV setup is also available as a preset. It uses `--capture-backend cpu` by default because the GL path can produce black frames on some Portal/PipeWire sessions:

```text
lumaway sync --bridge <ip> --area TV --preset tv-wayland
```

With environment variables set, the daily command becomes:

```text
lumaway sync --preset tv-wayland
```

For a long-running TV session:

```text
lumaway sync --preset tv-wayland --duration-ms 0
```

For daily use, install the desktop application:

```text
scripts/install-desktop-app.sh
```

Then launch `LumaWay` from the application menu. The GUI lets you associate the lighting controller, load zones, edit the connection, start or stop sync, and inspect logs. See [docs/desktop-app.md](docs/desktop-app.md).

Lighting channel positions are read from the controller; the current spike maps relative `position.x` and `position.y` ranges to normalized screen sample points with an 8% edge margin. If all positioned channels share the same vertical coordinate, the sampler uses the vertical center instead of pinning lights to the top or bottom edge. Channels without controller positions fall back to an even horizontal distribution.

The Portal/PipeWire path currently negotiates the native sample format from GStreamer caps and samples that buffer directly. `--sample-width` and `--sample-height` define a CPU sampling grid over the native buffer. The default sync grid is `120x68`. GStreamer-side Portal scaling is disabled for now because width/height caps caused frame timeouts in GNOME Portal testing.

`--sample-edge-margin` controls how far 2D sample points stay away from the exact screen edges. The default is `0.08`. Lower it if lights should react closer to screen borders.

`--sampling point|region` selects the spatial sampler. `point` samples a small patch around each channel anchor and is useful for comparisons. `region` samples a larger weighted rectangle around each channel anchor, so lights react to window-sized color changes instead of tiny pixels. The `tv-wayland` preset uses `region`.

`--sample-crop-left`, `--sample-crop-right`, `--sample-crop-top`, and `--sample-crop-bottom` constrain sampling to a content region before points are read. Use these for black bars or desktop areas that should not influence lighting output:

```text
lumaway sync --bridge <ip> --area <id> --sample-crop-top 0.12 --sample-crop-bottom 0.12
```

`--auto-crop` measures dark borders before stream activation and applies the detected crop during the sync run. Manual crop values are still honored, and the effective crop uses the larger value for each edge. Automatic crop is capped per edge by `--auto-crop-max-edge`, which defaults to `0.35`.

```text
lumaway sync --bridge <ip> --area <id> --auto-crop --auto-crop-frames 5 --auto-crop-threshold 8 --auto-crop-max-edge 0.35
```

Temporal smoothing is enabled by default, while the `tv-wayland` preset leaves `--max-step` disabled so window changes remain responsive. The sync path also applies a Hue-oriented color grade (gain, gamma lift, and saturation boost) before the final brightness scale, so dim captured windows still produce visible light output at `--brightness 1.0`.

```text
lumaway sync --bridge <ip> --area <id> --smoothing 0.35 --noise-threshold 3 --max-step 32
```

`--smoothing` is the current-frame weight. Lower values are calmer and slower; `1.0` disables smoothing.
`--noise-threshold` ignores per-channel RGB changes at or below the configured delta; `0` disables the threshold.
`--max-step` limits the maximum per-frame RGB channel change; omit it to disable this limiter.
`--color-profile soft|vivid|game|boosted|cinema|desktop` selects the color grading curve. The default is `vivid`; it can also be set with `LUMAWAY_COLOR_PROFILE`. Use `boosted` when capture quality is good but the measured or visible output remains undersaturated.

Capture and color defaults can be stored in non-secret profile files under `~/.config/lumaway/profiles/`. Create a starter profile:

```text
lumaway profile-template --name default
```

List available profiles:

```text
lumaway profile-list
```

Then set `LUMAWAY_PROFILE=default` in `~/.config/lumaway/lumaway.env` or your shell. The CLI loads `~/.config/lumaway/lumaway.env` automatically before reading profile defaults, so GUI-saved bridge credentials are the source of truth for commands run without explicit flags. Profile files are simple `key=value` files and may define `LUMAWAY_PRESET`, `LUMAWAY_CAPTURE_BACKEND`, `LUMAWAY_CAPTURE_FPS`, `LUMAWAY_STREAM_FPS`, `LUMAWAY_PIPEWIRE_FPS`, `LUMAWAY_CAPTURE_POLL_MS`, `LUMAWAY_SAMPLE_WIDTH`, `LUMAWAY_SAMPLE_HEIGHT`, `LUMAWAY_SAMPLE_EDGE_MARGIN`, `LUMAWAY_SAMPLING`, `LUMAWAY_BRIGHTNESS`, `LUMAWAY_REACTIVITY`, `LUMAWAY_COLOR_PROFILE`, `LUMAWAY_NOISE_THRESHOLD`, and `LUMAWAY_MAX_STEP`. Secrets stay in `lumaway.env`.

The desktop installer creates `~/.config/lumaway/profiles/default.env` automatically. The GUI Settings window has a `Capture profile` field that writes `LUMAWAY_PROFILE`, plus a `Calibrate` button that runs the capture calibration for the selected profile.

Generate a profile from a real Portal capture probe:

```text
lumaway calibrate-capture --name tv
```

This compares CPU and GL capture on the selected stream, writes `~/.config/lumaway/profiles/tv.env`, and records the measured backend result as comments at the top of the profile. Use `--force` to overwrite an existing profile.

Measure whether capture is bright, changing over time, and spatially different between Hue channels:

```text
lumaway capture-quality --portal --preset tv-wayland --area TV --frames 30
```

The summary reports average luma, saturation, per-frame RGB delta, channel separation, dark frames, and a recommendation such as `capture_too_dark`, `low_temporal_variation`, `low_spatial_separation`, `low_saturation`, or `usable`.

If `doctor` reports that the saved Hue application key was rejected, pair again from the GUI Settings window: press the physical bridge button, then press `Pair` in LumaWay. That refreshes `LUMAWAY_APP_KEY` and `LUMAWAY_CLIENT_KEY` in `~/.config/lumaway/lumaway.env`.

Inspect the capture-to-color pipeline without starting Hue streaming:

```text
lumaway sample-debug --portal --preset tv-wayland --frames 3
```

The command prints one row per Hue Entertainment channel with sample point, effective region, sample radius, raw RGB, smoothed RGB, graded RGB, final output RGB, luma, saturation, and capture timing. Use it before tuning a room or debugging a light that does not match the expected screen region.

At the end of a successful sync run, the CLI prints timing metrics:

```text
sync_stats capture_backend=<cpu|gl|unknown> interrupted=<true|false> frames=<n> capture_frames=<n> repeated_frames=<n> missed_capture_frames=<n> empty_capture_polls=<n> capture_avg_ms=<ms> capture_max_ms=<ms> color_avg_ms=<ms> color_max_ms=<ms> encode_avg_ms=<ms> encode_max_ms=<ms> send_avg_ms=<ms> send_max_ms=<ms>
```

`capture_backend` is the effective backend after `auto` fallback resolution. `interrupted` is true when shutdown came from Ctrl-C or SIGTERM. `frames` is the number of stream packets sent. `capture_frames` is the number of screen samples accepted, `repeated_frames` is the number of stream packets that reused the previous color state, `missed_capture_frames` is the gap between expected and accepted captures for the requested `--capture-fps`, and `empty_capture_polls` counts opportunistic polls that found no new Portal/PipeWire frame. These timings cover active work only. The frame pacing sleep is intentionally excluded.

Current GNOME Wayland + TV-zone baseline:

```text
sync_stats frames=250 capture_frames=129 repeated_frames=121 missed_capture_frames=0 empty_capture_polls=121 capture_avg_ms=12.001 capture_max_ms=20.074 color_avg_ms=0.003 color_max_ms=0.007 encode_avg_ms=0.001 encode_max_ms=0.007 send_avg_ms=0.015 send_max_ms=0.040
```

Compare sampling grids without network streaming:

```text
lumaway sample-bench --portal --frames 20 --bands 2 --grids 80x45,120x68,160x90,240x135
```

Benchmark the sync loop without credentials or network I/O:

```text
lumaway sync-bench --duration-ms 10000 --capture-fps 8 --stream-fps 25 --pipewire-fps 25 --capture-backend gl --capture-poll-ms 5
```

Current GNOME Wayland sample benchmark:

```text
sample_bench grid=80x45 frames=20 bands=2 capture_avg_ms=31.637 capture_max_ms=48.311 sample_avg_ms=0.312 sample_max_ms=0.355
sample_bench grid=120x68 frames=20 bands=2 capture_avg_ms=31.637 capture_max_ms=48.311 sample_avg_ms=0.309 sample_max_ms=0.348
sample_bench grid=160x90 frames=20 bands=2 capture_avg_ms=31.637 capture_max_ms=48.311 sample_avg_ms=0.532 sample_max_ms=0.757
sample_bench grid=240x135 frames=20 bands=2 capture_avg_ms=31.637 capture_max_ms=48.311 sample_avg_ms=0.769 sample_max_ms=0.912
```

Detect dark borders without network streaming:

```text
lumaway detect-crop --portal --frames 5 --threshold 8 --sample-width 120 --sample-height 68 --max-edge 0.35
```

The command prints per-frame crop estimates, a `crop_suggested` summary, and a `crop_args` line that can be copied into `lumaway sync`. `--max-edge` applies the same per-edge cap as `sync --auto-crop-max-edge`; omit it to inspect the raw detection. For one-shot automatic use, pass `--auto-crop` directly to `lumaway sync`.

Current GNOME Wayland crop diagnostic baseline:

```text
crop_suggested frames=5 left=0.0000 right=0.0000 top=0.0147 bottom=0.0000
crop_args --sample-crop-left 0.0000 --sample-crop-right 0.0000 --sample-crop-top 0.0147 --sample-crop-bottom 0.0000
```

Ask the desktop portal for a screen/window stream and print the PipeWire node:

```text
lumaway portal-probe
```

`LUMAWAY_APP_KEY` and `LUMAWAY_CLIENT_KEY` can be used instead of passing secrets as CLI arguments.

This sync path is still a spike. It has 2D spatial point and weighted-region sampling, manual crop controls, optional dark-border auto-crop, temporal smoothing, a small anti-noise threshold, and an optional per-frame step limiter, but not yet full 3D-aware placement, persistent crop profiles, or latency tuning.

## Documentation

| Document | Description |
|----------|-------------|
| [docs/plan-hue-sync-daily.md](docs/plan-hue-sync-daily.md) | **Product roadmap (référence v1)** — TOC + guide de lecture en tête ; modes §6, release §15.2 |
| [docs/desktop-app.md](docs/desktop-app.md) | GTK/libadwaita install, config, and validation |
| [docs/hue-sync-research.md](docs/hue-sync-research.md) | Public Hue Sync / Entertainment API research notes |
| [docs/capture-improvement-roadmap.md](docs/capture-improvement-roadmap.md) | Capture-to-color quality improvements |
| [docs/security.md](docs/security.md) | TLS pinning, DTLS LAN policy, credentials |
| [docs/backlog.md](docs/backlog.md) | Milestone history and deferred items |
| [docs/test-matrix.md](docs/test-matrix.md) | Wayland / Portal / Hue test scenarios |

## Workspace

```text
crates/
  lumaway-hue/   local lighting REST and streaming protocol
  lumaway-core/  Portal, PipeWire, and GStreamer capture pipeline
  lumaway-cli/   Headless CLI for spikes and diagnostics
```

Planned later:

```text
crates/
  lumaway-config/
  lumaway-gtk/
```

## Security

TLS to the Hue bridge uses relaxed certificate verification (self-signed bridge certs). DTLS for entertainment is PSK-based without X.509 pinning. See [docs/security.md](docs/security.md) for the trust model, LAN considerations, and GUI-related controls (`LUMAWAY_BIN`, discovery).

## License

MPL-2.0.
