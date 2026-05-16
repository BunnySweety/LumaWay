# Capture improvement roadmap

Date: 2026-05-16

Aligné avec la **Phase 2** du plan produit : [plan-hue-sync-daily.md](plan-hue-sync-daily.md).


## Goal

Improve LumaWay capture and color rendering so Hue lights react visibly, spatially, and predictably when windows or screen content change.

Current symptoms to address:

- brightness at 100% can still feel dim;
- color variations can be too weak;
- light colors do not always feel correlated with the window or screen region near each light;
- smoothing and max-step limiting can make window changes feel delayed or muted;
- the capture backend can affect correctness: in the current GNOME Wayland session, CPU capture produced usable frames while the GL path could produce black frames.

## Baseline Direction

Keep the current Linux Wayland architecture:

- XDG Desktop Portal for user-approved screen/window capture;
- PipeWire/GStreamer for frame delivery;
- Hue Entertainment API over DTLS for low-latency streaming;
- CPU capture as the `video-wayland` Video default (`tv-wayland` remains a legacy alias) until the GL path is proven reliable across sessions;
- one active Hue Entertainment area at a time.

The improvement work should focus on the capture-to-color pipeline:

1. capture correctness;
2. spatial sampling;
3. color grading;
4. temporal behavior;
5. diagnostics and calibration.

## Source Projects To Study

### Hyperion

Hyperion is the strongest open-source reference for ambilight behavior. Its Hue documentation confirms practical constraints that match LumaWay:

- light color depends on input signal, screen position, and capture frame rate;
- Hue Entertainment areas are required for low-latency streaming;
- the active Entertainment area owns the lights while streaming;
- signal detection and black-level handling matter.

Reference:

- https://docs.hyperion-project.org/user/leddevices/network/philipshue.html

What to extract:

- LED or lamp-to-screen area mapping;
- edge-region sampling instead of single-point sampling;
- black/signal detection;
- color calibration model;
- profile-based behavior for video, game, and desktop modes.

### Glimmr

Glimmr is a multi-device ambient lighting project supporting Hue, Lifx, Nanoleaf, WLED, OpenRGB, HDMI input, webcam, and screen capture.

Reference:

- https://github.com/d8ahazard/glimmr

What to extract:

- source abstraction: screen, HDMI, webcam;
- multi-device color pipeline structure;
- device-independent color processing;
- UI/config model for profiles and calibration.

### LumaSync

LumaSync is a newer open-source desktop ambient-lighting app for Philips Hue and LED strips with Linux marked as experimental.

Reference:

- https://lumasync.app/

What to extract:

- desktop capture strategy;
- Linux-specific limitations;
- color mapping and user-facing tuning controls;
- whether it separates capture FPS from output FPS.

### Hue Entertainment Repositories

GitHub topic:

- https://github.com/topics/hue-entertainment

What to extract:

- DTLS handshake handling;
- HueStream packet cadence;
- bridge quirks;
- identity/client-key handling;
- recovery behavior when the bridge stops entertainment.

## Reverse Engineering Strategy

Avoid broad reverse engineering of the closed Hue Sync app at first. The fastest useful path is behavioral reverse engineering through controlled visual tests.

### Useful Reverse Engineering

Build a repeatable comparison harness:

1. display known test patterns on screen;
2. run official Hue Sync on Windows/macOS if available;
3. record visible light output or query measurable Hue state where possible;
4. run LumaWay with the same pattern;
5. compare timing, luminance, saturation, and spatial mapping.

Patterns to use:

- full-screen red, green, blue, white, black;
- left/right split colors;
- top/bottom split colors;
- moving colored window;
- small bright object on dark background;
- dark movie-like frame;
- saturated UI window on neutral desktop;
- black bars/letterbox content.

Metrics to record:

- first visible reaction latency;
- steady-state color per light;
- brightness floor on dark content;
- saturation on colored content;
- fade behavior when content changes;
- black handling;
- whether neighboring lights differ when content differs spatially.

### Lower Priority Reverse Engineering

Only consider deeper protocol or app reverse engineering if behavioral comparison is insufficient.

Potential targets:

- Hue Sync gamma curve;
- Hue Sync saturation/vibrance curve;
- mode-specific behavior: video, game, music;
- black-frame behavior and signal-loss timeout;
- spatial weighting around each light.

Do not depend on private APIs or undocumented bridge behavior unless there is no stable public alternative.

## Capture Improvements

### 1. Add A Sample Debug Command

Add a command like:

```text
lumaway sample-debug --portal --sync-mode video --preset video-wayland
```

It should print one row per channel:

- channel id;
- normalized sample point;
- effective sample region;
- raw RGB;
- smoothed RGB;
- graded RGB;
- output RGB;
- luma;
- saturation;
- capture backend;
- frame timing.

This gives direct evidence for reports like "the left light does not match the left window".

### 2. Region-Based Sampling

Current point sampling is too fragile for window changes. Replace or supplement it with region sampling:

- left lights sample a left edge rectangle;
- right lights sample a right edge rectangle;
- top lights sample a top rectangle;
- bottom lights sample a bottom rectangle;
- center/no-position lights sample broader fallback regions;
- each light gets a weighted region, not one point.

Possible model:

- derive each channel's normalized Hue Entertainment position;
- map it to an anchor on the screen;
- build an elliptical or rectangular sample region around that anchor;
- weight pixels closer to the anchor more strongly;
- expose region size as a profile parameter.

Expected effect:

- stronger correlation between lights and windows;
- less sensitivity to tiny dark UI details;
- smoother but still spatially accurate output.

### 3. Improve Color Grading

Maintain separate stages:

1. raw captured RGB;
2. spatial average;
3. smoothing;
4. color grading;
5. final brightness scale;
6. HueStream RGB encode.

Color grading should include:

- gain;
- gamma;
- saturation;
- optional vibrance;
- black floor;
- minimum output brightness for non-black content;
- white clamp to avoid washing everything out.

Initial profiles:

- `soft`: low saturation, slower changes;
- `vivid`: higher gain and saturation;
- `game`: fast response, limited smoothing;
- `cinema`: black-aware, less flicker;
- `desktop`: avoids overreacting to small bright UI elements.

### 4. Better Temporal Behavior

Separate these concepts:

- smoothing: reduce flicker;
- max step: limit abrupt jumps;
- reactivity: user-facing speed control;
- signal detection: decide when content is actually black or absent.

The current Video preset (`video-wayland`, with `tv-wayland` as a legacy alias) should stay responsive:

- no default max-step limiter;
- moderate smoothing;
- low noise threshold;
- repeated stream frames when capture has no fresh frame.

Add future tuning:

- faster response for large scene changes;
- slower smoothing for minor noise;
- immediate transition when switching windows and color delta is large.

### 5. Capture Backend Quality Checks

Keep CPU as the daily default until GL is validated.

Add a backend self-test:

```text
lumaway backend-probe
```

- capture a few frames;
- compute max RGB and average luma;
- if GL returns repeated black frames while CPU does not, fall back to CPU;
- log the decision clearly.

This avoids black-output sessions when the GL path technically starts but returns unusable frames.

### 6. Calibration Workflow

Add a calibration command:

```text
lumaway calibrate-capture --portal
```

It should guide through known patterns:

- red;
- green;
- blue;
- white;
- black;
- split left/right;
- split top/bottom.

The output should be a profile file with:

- capture backend;
- sample crop;
- gain;
- gamma;
- saturation;
- black threshold;
- region size;
- smoothing defaults.

Store profiles separately from credentials.

## Implementation Plan

## Implementation Status

- Done: `lumaway sample-debug` prints per-channel sample points, sample radius, raw RGB, smoothed RGB, graded RGB, final output RGB, luma, saturation, capture backend, and capture timing without starting Hue streaming.
- Done: the point-sampling patch was widened to reduce single-pixel/detail sensitivity.
- Done: `SampleRegion` and `--sampling point|region` add weighted rectangular region sampling while keeping point sampling available for comparison.
- Done: 2D channel projection uses Hue `position.y` first and falls back to `position.z` as vertical placement when the bridge exposes depth variation but no usable vertical span.
- Done: `--color-profile soft|vivid|game|boosted|cinema|desktop` and `LUMAWAY_COLOR_PROFILE` select color grading curves for `sync`, `sync-bench`, and `sample-debug`.
- Done: the default Video / `vivid` curve keeps true black and near-black capture noise dark while applying a soft minimum output luma to dim non-black content.
- Done: the GTK Settings window exposes the same color-profile selector and passes it to the sync engine.
- Done: `video-wayland` and the legacy `tv-wayland` alias use CPU capture by default, weighted region sampling, and keep max-step disabled for more responsive window changes.
- Done: `lumaway backend-probe` compares CPU and GL on the same Portal stream and reports frames, max RGB, average luma, dark-frame detection, timing, and a conservative recommendation.
- Done: the GUI offers a `Probe backend` assistant action after capture-too-dark failures and summarizes the CPU/GL recommendation in the app log.
- Done: `--capture-backend auto` now probes GL output quality and falls back to CPU when GL starts but returns black/unusable frames.
- Done: Phase 2.4 comparison harness is documented in `phase2-comparison-harness.md` with a local `fixtures/phase2-patterns.html` full-screen pattern page and a 300 ms latency gate.
- Done: `lumaway capture-quality` summarizes real Portal capture luma, saturation, temporal variation, channel separation, dark frames, and a recommendation for weak capture symptoms.
- Done: capture quality distinguishes one-channel areas from real spatial-correlation failures and tells the user to test with a multi-light area.
- Done: `LUMAWAY_PROFILE=<name>` loads non-secret capture/color defaults from `~/.config/lumaway/profiles/<name>.env`; `lumaway profile-template --name <name>` creates a starter profile and `lumaway profile-list` lists available profile files.
- Done: profiles can persist manual crop values with `LUMAWAY_SAMPLE_CROP_LEFT`, `LUMAWAY_SAMPLE_CROP_RIGHT`, `LUMAWAY_SAMPLE_CROP_TOP`, and `LUMAWAY_SAMPLE_CROP_BOTTOM`; `sync`, `sample-debug`, and `capture-quality` read them from the selected profile unless explicit CLI flags override them.
- Done: CLI commands automatically load `~/.config/lumaway/lumaway.env` before profile defaults, so diagnostics can reuse the GUI-saved bridge, keys, area, and selected profile without manual shell sourcing.
- Done: `lumaway calibrate-capture --name <name>` probes CPU/GL on a real Portal stream and writes a measured profile with the recommended capture backend.
- Done: the GUI Settings window exposes a `Capture profile` field, can list existing profiles, passes `LUMAWAY_PROFILE` to sync, and has a `Calibrate` button that writes the selected measured profile.
- Post-v1.0 / non-blocking for the Phase 2 release gate: guided pattern-based calibration for color/gamma/saturation and a richer GUI profile picker/dropdown.

### Phase 1: Diagnostics

- Add `sample-debug`.
- Log per-channel raw and graded colors on demand.
- Add a static synthetic-frame test for region sampling and color grading.
- Add a live smoke command that runs capture without Hue streaming.

Exit criteria:

- it is possible to prove what each light would receive before touching real Hue lights.

### Phase 2: Region Sampling

- Introduce `SampleRegion`.
- Generate regions from Hue channel positions.
- Keep point sampling available for comparison.
- Add tests for left/right/top/bottom region mapping.
- Add a CLI option:

```text
--sampling point|region
```

Exit criteria:

- moving a colored window from left to right changes the intended channel rows in `sample-debug`.

### Phase 3: Color Profiles

- Introduce a color profile struct.
- Move gain/gamma/saturation/black-floor values out of hard-coded constants.
- Add presets:

```text
--color-profile soft|vivid|game|boosted|cinema|desktop
```

Exit criteria:

- dim captured colors can be boosted without making bright content permanently white.

### Phase 4: Backend Validation

- Add GL-vs-CPU startup probe.
- Fall back from GL to CPU if GL returns black/unusable frames.
- Record backend decision in `sync_stats`.

Exit criteria:

- `auto` is safe again because it validates output quality, not just pipeline startup.

### Phase 5: Calibration

- Add `calibrate-capture`.
- Persist capture/color profiles.
- Expose profile selection in the GUI.

Exit criteria:

- a user can generate a stable per-room/per-monitor profile without editing CLI flags.

## Open Questions

- Should LumaWay support per-light manual region overrides?
- Should profile files be TOML, JSON, or part of the existing env/config model?
- Should GUI expose advanced controls directly or hide them behind profile presets?
- How should HDR or protected content be detected when Portal capture returns dark frames?
- Should black-frame handling restore the previous Hue state after a timeout, like some ambilight systems do?

## Near-Term Recommendation

Build `sample-debug` first. It is the foundation for every later improvement because it turns subjective visual complaints into measurable data:

- what did the capture see?
- which screen region was assigned to each light?
- what color did the grading produce?
- what was sent to Hue?

Region sampling, color profiles, backend quality self-test, CPU fallback, persistent capture profiles, and GUI profile wiring are now implemented. The remaining Phase 2 release gate is external field evidence: record the comparison harness with screen and Hue lights, confirm visible latency at or below 300 ms for at least five accepted transitions, confirm no silent black session, and time a no-calibration first run at 10 minutes or less.
