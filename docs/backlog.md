# Initial Backlog

**Feuille de route produit (Hue Sync quotidien + musique)** : [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

## Milestone 0: Hue DTLS Spike

1. Create `lumaway-hue` REST client skeleton. Done.
2. Implement bridge user creation flow. Done.
3. List Hue Entertainment Areas. Done.
4. Fetch one Entertainment Area by ID. Done.
5. Build HueStream frame encoder tests. Done.
6. Prototype DTLS connection with OpenSSL bindings. Done.
7. Implement `lumaway test-color --bridge <ip> --area <id> --color red`.
   - REST activation is wired.
   - HueStream RGB frame construction is wired.
   - DTLS transport is wired as a spike.
   - Real Hue hardware validation is done against bridge `192.168.1.108`, area `TV`.
8. Add structured tracing output. Done for CLI spike.
9. Document DTLS findings in an ADR.
10. Add `lumaway doctor` for initial environment checks. Done.

## Milestone 1: Capture Spike

1. Select `ashpd` or `zbus` for ScreenCast portal access. `ashpd` selected first.
2. Prototype portal session creation. Done via `lumaway portal-probe`.
   - Validated in GNOME Wayland session as user `bunny`.
   - Returned PipeWire node `107`, size `2560x1440`, position `(2560, 0)`.
3. Prototype GStreamer appsink frame retrieval. Done with `videotestsrc` and portal PipeWire.
4. Add `lumaway capture-stats`. Done for GStreamer smoke test and portal capture.
   - `videotestsrc`: 61 frames in 2001 ms, about 30.48 FPS.
   - portal PipeWire: 65 frames in 2029 ms, about 32.03 FPS.
   - portal PipeWire with forced RGB caps: 9 frames in 2016 ms, about 4.46 FPS.
   - `videotestsrc` scaled RGB profile `160x90@25`: 26 frames in 1001 ms, about 25.96 FPS.
5. Collect FPS and stage timing metrics. Basic FPS done; stage timings remain.

## Milestone 2: Sync Spike

1. Add average RGB extraction from captured frames. Done.
2. Add RGB/XY conversion.
3. Add channel mapping. Initial 2D spatial mapping done.
   - Hue Entertainment channel positions are parsed from the bridge response.
   - Channels are sorted by `position.x` when available, with `channel_id` fallback.
   - Relative `position.x` and `position.y` ranges are mapped to normalized screen sample points.
   - Sample points use a configurable edge margin with `--sample-edge-margin`; default is 8%.
   - Sample points can be constrained with manual crop controls: `--sample-crop-left`, `--sample-crop-right`, `--sample-crop-top`, and `--sample-crop-bottom`.
   - If `position.y` has no usable vertical span, the sampler uses Hue Entertainment `position.z` as the vertical fallback; if neither axis has a usable span, it uses vertical center.
   - Channels without positions fall back to an even horizontal distribution.
4. Add backpressure rules.
5. Add temporal smoothing. Done.
   - `ColorSmoother` applies exponential smoothing per channel.
   - `lumaway sync --smoothing <0.0..1.0>` controls the current-frame weight.
   - Default smoothing is `0.35`; `1.0` disables smoothing.
   - `lumaway sync --noise-threshold <0..255>` suppresses small per-channel RGB deltas.
   - Default noise threshold is `3`; `0` disables the threshold.
   - `lumaway sync --max-step <0..255>` limits per-frame RGB channel changes.
   - `--max-step` is optional and disabled by default.
6. Add sync timing metrics. Done.
   - `sync_stats` reports average and max timings for capture, color pipeline, encode, and send.
   - `sync_stats` also reports Hue stream frames, captured frames, repeated frames, missed target captures, and empty opportunistic capture polls.
   - Frame pacing sleep is excluded from timing metrics.
7. Add `lumaway sync --bridge <ip> --area <id>`. Initial portal average sync added.
   - Hardware smoke test on `192.168.1.108`, area `TV`, completed with 16 HueStream frames sent.
   - Portal capture path now negotiates native GStreamer sample caps and supports common RGB/BGR layouts.
   - GStreamer-side Portal scaling is disabled for now: width/height caps caused frame timeouts.
   - `--sample-width` and `--sample-height` now define a CPU sampling grid over the native buffer.
   - Default sync sampling grid is now `120x68`.
   - Hardware sync baseline on `192.168.1.108`, area `TV`, completed with 37 HueStream frames sent.
   - Baseline timings: capture avg 34.954 ms / max 36.083 ms, color avg 0.007 ms, encode avg 0.004 ms, send avg 0.022 ms.
   - CPU-sampled baseline at `160x90` completed with 41 HueStream frames sent.
   - CPU-sampled timings: capture avg 20.943 ms / max 22.847 ms, color avg 0.007 ms, encode avg 0.004 ms, send avg 0.021 ms.
   - Initial 2D point-sampled baseline at `120x68` completed with 48 HueStream frames sent.
   - 2D point-sampled timings: capture avg 3.032 ms / max 20.545 ms, color avg 0.007 ms, encode avg 0.006 ms, send avg 0.029 ms.
   - Sync loop now preallocates sample points, channel colors, and the HueStream encoder buffer to avoid per-frame allocations in the color/encode path.
   - `sample-bench` added to compare CPU sampling grids in one Portal session without Hue.
   - Grid benchmark: `80x45` sample avg 0.312 ms, `120x68` avg 0.309 ms, `160x90` avg 0.532 ms, `240x135` avg 0.769 ms.
   - `detect-crop` added to estimate dark borders from Portal or videotest frames without Hue.
   - `sync --auto-crop` added to apply detected dark-border crop before Hue activation.
   - `--auto-crop-max-edge` caps automatic per-edge crop; default is 35%.
   - Sync cadence split added: `--capture-fps` controls Portal/PipeWire sampling, `--stream-fps` controls HueStream sends, and `--fps` still sets both when specific flags are omitted.
   - `--capture-poll-ms` added to tune non-initial capture polling without recompiling.
   - `--pipewire-fps` added; by default the Portal/PipeWire pipeline now runs at `max(capture-fps, stream-fps)` so a low sampling cadence does not throttle upstream frame delivery.
   - Stable real Hue validation on this GNOME Wayland session now uses auto capture through `--preset tv-wayland`; auto tries GL first and falls back to CPU. The GL path completed 10 seconds with 250 stream frames, 129 accepted capture frames, 0 missed target captures, and send max 0.040 ms.
   - CPU vs GL Portal benchmark showed GL improving accepted capture frames from 92 to 143 over the same 10 second run.
   - Sync now waits for a first real capture frame before starting HueStream timing, avoiding black-frame runs when Portal selection produces no initial buffer.
   - `sync --preset tv-wayland` added for the validated GNOME Wayland TV settings.
   - `doctor` now checks Portal D-Bus, user services, session environment, required GStreamer elements, and optional GL acceleration elements.
   - `LUMAWAY_BRIDGE` and `LUMAWAY_AREA` added so daily use can run as `lumaway sync --preset tv-wayland` when credentials are also exported.
   - `--duration-ms 0` added for long-running sessions until Ctrl-C, while preserving the normal Hue Entertainment deactivation path.
   - A desktop application path was added as the daily workflow: application menu entry, GTK/libadwaita window, visible logs, and Stop-button shutdown.

## Milestone 3: Desktop App Shell

1. Add GTK/libadwaita UI. Done.
   - `lumaway-gui` provides a native window with bridge, area, app key, client key, duration, Start, Stop, status, and log output.
   - GUI actions can create Hue credentials and load Entertainment Areas without leaving the application.
   - The UI was reorganized into clear setup, area, and sync sections with French labels, friendlier status text, disabled controls while syncing, and scrollable content for smaller screens.
   - The GUI persists local credentials in `~/.config/lumaway/lumaway.env` with mode `0600`.
   - Start/Stop controls the validated `lumaway sync --preset tv-wayland` engine.
   - GUI autostart/auto-quit validation mode covers the real Start path against Hue and Portal.

## Deferred

Voir aussi la section « Éléments reportés » dans [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

- Flatpak manifest (permissions Portal + audio — Phase 4 du plan).
- Secret Service storage.
- Migration from Lumux.
- Multi-monitor polish (Phase **1.10** du plan — flux Portal).
