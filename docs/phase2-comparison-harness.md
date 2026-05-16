# Phase 2 Comparison Harness

This harness validates the Phase 2 screen-quality goal: LumaWay should react visibly and spatially to known screen patterns without a silent black session.

## Scope

Use this for Phase 2.4 release evidence:

- fixed full-screen patterns: black, white, red, green, blue, left/right split, top/bottom split, dark movie frame, moving window, latency flash;
- per-channel color sanity through `sample-debug`;
- capture-quality sanity through `capture-quality`;
- visible reaction latency threshold for full-screen changes.

Out of scope:

- reverse engineering Philips Hue Sync binaries;
- protocol-level comparison with private Signify behavior;
- audio / Music mode.

## Fixture

Open the local pattern fixture:

```text
docs/fixtures/phase2-patterns.html
```

Use a browser window on the target display, press `F` for fullscreen, and choose that screen/window in the Portal selector. Use Left/Right or Space to switch patterns. Press `H` to hide the overlay before taking measurements.

## Baseline Commands

Run these from a graphical Wayland session with the target Entertainment zone configured:

```sh
lumaway backend-probe --frames 5 --sample-width 120 --sample-height 68 --fps 25
lumaway capture-quality --portal --sync-mode video --preset video-wayland --frames 30
lumaway sample-debug --portal --sync-mode video --preset video-wayland --area TV --frames 1 --sampling region --color-profile vivid
```

For a short live run:

```sh
lumaway sync --sync-mode video --preset video-wayland --area TV --duration-ms 10000
```

Replace `TV` with the configured Entertainment zone when needed.

For the internal latency guard, keep the final `sync_stats` line from the live run. At 25 Hz, the
stream frame interval is 40 ms. The internal budget is considered healthy when:

```text
capture_max_ms + send_max_ms + 80 ms < 300 ms
```

This is only a pipeline sanity check. It does not replace the visible light-response video below,
because Hue bridge/light processing and camera-visible emission happen after LumaWay sends a frame.

## Expected Results

| Pattern | Expected evidence |
|---------|-------------------|
| Black | No silent crash; output is black or near off. |
| White | All channels converge toward white. |
| Red / Green / Blue | All channels converge toward the selected primary color. |
| Left red / right blue | Left-positioned channels are red-biased; right-positioned channels are blue-biased in `sample-debug`. |
| Top white / bottom black | Top-positioned channels are brighter than bottom-positioned channels when the bridge exposes usable Y positions, or usable Z positions when Y is flat. |
| Dark movie frame | Not classified as a fully black session unless the frame is actually black; Video / `vivid` should show a soft low-luma response. |
| Moving red window | Red output moves from left channels toward right channels as the window moves. |
| Latency flash | First visible light response follows a full-screen flash within the threshold below. |

## Latency Threshold

Use the `Latency flash` pattern. Record the screen and the Hue lights in the same video at 120 fps or higher. Measure from the first frame where the screen changes to the first frame where any target light visibly changes.

Before starting the manual run, use the preflight helper on the target machine:

```sh
scripts/phase2-field-preflight.sh | tee phase2-preflight.txt
```

It checks that the Phase 2 helpers, harness files, and a `/dev/video*` camera are present. When using a phone or another non-V4L2 camera, declare the planned capture rate and skip the local device requirement:

```sh
scripts/phase2-field-preflight.sh --require-camera no --camera-fps 120 | tee phase2-preflight.txt
```

The helper checks declared camera FPS directly, or V4L2-reported FPS when `v4l2-ctl` is available for a local `/dev/video*` camera. It does not replace the measured evidence below.

Phase 2.4 / v1.0 pass criteria:

- measure at least 5 full-screen black-to-white or white-to-black transitions;
- every accepted transition must be at or below 300 ms;
- no session may stay black silently after a non-black pattern is shown;
- if a transition is discarded, record why, for example camera exposure loss or occluded light.

After reading frame numbers from the video, use the helper to avoid manual arithmetic mistakes:

```sh
scripts/phase2-latency-summary.sh --fps 120 100:124 220:247 340:369 460:490 580:613
```

Each argument is `screen_frame:light_frame`. The command exits non-zero if fewer than 5
transitions are supplied or if any accepted transition exceeds 300 ms.

## First-Run Timing

For the Phase 2 finish criterion, time a user who starts from an installed app and no prior capture
calibration. Start the timer when they launch LumaWay. Stop it when the selected TV/monitor zone
is syncing a non-black screen with acceptable spatial response.

Pass criteria:

- existing TV user: sync starts without running `calibrate-capture`;
- new user: first satisfactory sync is reached in 10 minutes or less;
- if the bridge has no Entertainment zone, record that as environment setup time outside LumaWay;
- if `Probe backend` or `Quality` is used, record it as part of the timed flow.

After measuring the elapsed time, use the helper for the Phase 2 pass/fail verdict:

```sh
scripts/phase2-first-run-summary.sh --elapsed-seconds 420 --calibrate-used no
```

The command exits non-zero if elapsed time is greater than 600 seconds or if
`calibrate-capture` was used.

To generate one pasteable Phase 2 field-evidence block after both manual
measurements are available, run:

```sh
scripts/phase2-field-evidence.sh --fps 120 --elapsed-seconds 420 --preflight-output phase2-preflight.txt --calibrate-used no --silent-black no 100:124 220:247 340:369 460:490 580:613
```

This command reuses the latency and first-run helpers above. It exits non-zero
if the saved preflight is missing, ambiguous, or did not pass, both preflight
options are provided, the recording frame rate is below 120 fps, latency,
first-run timing, or the no-silent-black gate fails. With a valid preflight file
and valid arguments, it still prints every evidence section so the failed result
can be copied into the validation audit; missing, ambiguous, or conflicting
preflight inputs are treated as command errors.

## Result Template

```text
date:
desktop/session:
bridge firmware:
zone:
profile:
capture backend recommendation:
sync command:
sync_stats internal latency guard:
first-run timer start:
first satisfactory sync:
first-run elapsed:
calibrate-capture used: yes/no
silent black session observed: yes/no

pattern                  pass/fail   evidence
black
white
red
green
blue
left red / right blue
top white / bottom black
dark movie frame
moving red window

latency transition       ms          pass/fail
1
2
3
4
5

notes:
```

Commit any threshold or preset change only with this result block filled in, so future tuning is anchored to repeatable evidence.
