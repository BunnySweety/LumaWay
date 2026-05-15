# Hue Sync research notes

Date: 2026-05-10

**Plan d’exécution produit** : [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

## What is publicly known

- Philips Hue Sync is closed-source. Public Signify material says the desktop app synchronizes Hue lights with games, video, and music on Windows and macOS, with user-facing modes for Audio, Video, and Gaming plus brightness and intensity/speed controls.
- The Hue Sync TV app terms say the app accesses data related to color points on the screen to provide the lighting experience. This supports a point/zone sampling model, not full-frame pixel analysis for every light update.
- The Hue Entertainment API is separate from normal REST light control. It uses local UDP streaming secured by DTLS with a PSK client key.
- Public Entertainment API writeups describe the HueStream message shape as a fixed header, a 36-byte entertainment configuration id, then 7 bytes per channel.
- Public writeups disagree on practical cadence: one describes the bridge/lights as effectively 25 Hz; another notes Philips documentation recommending 50-60 Hz to the Bridge while the Bridge forwards over Zigbee around 25 Hz. The safest design is therefore to support low-latency capture while allowing repeated/continuous output without assuming every capture produces a new Zigbee update.
- Hyperion's Hue documentation reinforces several practical constraints:
  - Entertainment areas are required; normal Hue groups are not enough.
  - Only one entertainment area can be active per Bridge at a time.
  - Up to 10 color-capable devices are documented per Entertainment area.
  - Light position and capture frame rate affect perceived color behavior.
  - Smoothing is important to reduce fast color flicker.

## Implications for LumaWay

- Keep the current Wayland Portal + PipeWire path: it is the correct Linux Wayland access model.
- Keep point/zone sampling rather than whole-frame analysis for the real-time path.
- Keep temporal smoothing, noise thresholding, and optional max-step limiting.
- Keep manual and automatic crop controls; public sources and real Portal tests show screen edge/content region handling matters.
- Avoid allocation in the sync loop. HueStream messages are small but frequent, so the hot path should reuse sample points, channel buffers, and encoded message memory.
- Capture cadence and stream cadence should remain independently tunable:
  - capture/sample at a stable rate tuned for CPU and Portal behavior;
  - stream at a configurable rate, repeating the last frame when capture has not produced a new sample.

## Changes already applied

- Added reusable `HueStreamEncoder`.
- Added `DtlsTransport::send_bytes`.
- Updated `sync` to preallocate sample points, channel colors, and HueStream message bytes outside the frame loop.
- Added `--capture-fps` and `--stream-fps`; `--fps` remains a compatibility shortcut for both cadences.
- Added `--pipewire-fps`; by default the Portal/PipeWire capture pipeline runs at the highest configured cadence instead of being throttled by the sampling cadence.
- Added `--capture-poll-ms` to tune non-initial capture polling when Portal/PipeWire delivers frames slightly after the stream tick.
- Kept `sync_stats` timing for capture, color, encode, and send stages, and added frame counters for stream frames, captured frames, repeated frames, missed target captures, and empty opportunistic capture polls.

## Sources

- Signify Hue Sync press release: https://www.signify.com/en-us/our-company/news/press-releases/2018/20180531-create-surround-sound-for-your-eyes-with-philips-hue-sync
- Philips Hue Sync TV App terms: https://www.assets.signify.com/is/content/Signify/Assets/hue/global/legal/new/20240401-philips-hue-sync-tv-app-terms-version-april-2024-english.pdf
- Philips Hue Entertainment API overview: https://iotech.blog/posts/philips-hue-entertainment-api/
- Hyperion Philips Hue documentation: https://docs.hyperion-project.org/user/leddevices/network/philipshue.html
- HueCommand Entertainment streaming overview: https://huecommand.com/hue-entertainment-streaming
- Razer Philips Hue module guide: https://dl.razerzone.com/master-guides/PhilipsHueModule/PhilipsHueModulev2-en.pdf
