# LumaWay desktop app

This is the preferred development path for daily Linux Wayland use.

**Roadmap** (parité UX Hue Sync, **i18n**, modes, tray, release v1.0 écran quotidien, puis musique) : [plan-hue-sync-daily.md](plan-hue-sync-daily.md) — §3 (UX), §3.5 (i18n), §15 (lacunes & critères release).

LumaWay starts as a normal GTK/libadwaita desktop application. The GUI is organized around three steps: connect the local lighting controller, choose a configured lighting zone, then start or stop sync. It can detect the controller automatically, create local credentials after the controller link button is pressed, automatically load zones from the controller, and run the existing sync engine with:

```text
lumaway sync --sync-mode video --preset video-wayland --duration-ms 0
```

Use the Stop button to stop the sync cleanly.

## First launch

1. Start LumaWay from the application menu.
2. Let discovery fill the bridge address, or enter it in Settings.
3. Press the physical bridge button, then use Pair.
4. Load/select the Entertainment zone and run the red light test.
5. Keep mode Video for the v1.0 screen workflow, then Start sync.

## Install

From the repository root:

```text
scripts/install-desktop-app.sh
```

The script:

- builds `lumaway` and `lumaway-gui` in release mode;
- installs the binary to `~/.local/bin/lumaway`;
- installs the GUI to `~/.local/bin/lumaway-gui`;
- installs the desktop entry to `~/.local/share/applications/io.github.BunnySweety.LumaWay.desktop`;
- creates `~/.config/lumaway/lumaway.env` from the example if it does not exist;
- creates `~/.config/lumaway/profiles/default.env` from the default non-secret profile if it does not exist;
- keeps the environment file mode at `0600`.

Edit the local environment file:

```text
~/.config/lumaway/lumaway.env
```

Required values:

```text
LUMAWAY_BRIDGE=192.168.1.108
LUMAWAY_BRIDGE_ID=001788fffe123456
LUMAWAY_AREA=TV
LUMAWAY_APP_KEY=...
LUMAWAY_CLIENT_KEY=...
```

`LUMAWAY_BRIDGE_ID` is optional in the example file; LumaWay fills it automatically after pairing, zone load (`bridge-info`), or `lumaway bridge-info`. It is used for Hue HTTPS certificate pinning under `hue-tls-pins/by-id/` when `LUMAWAY_HUE_PIN_CERTS=1` (see [docs/security.md](security.md)).

Do not put real local credentials in Git.

The GUI detects the controller automatically at startup when no bridge address is configured. It automatically fetches configured zones when `LUMAWAY_BRIDGE` and `LUMAWAY_APP_KEY` are already available, preserving the saved zone if it is present in the returned list. The association action fills `LUMAWAY_APP_KEY` and `LUMAWAY_CLIENT_KEY` after the physical link button is pressed, then refreshes the zone list automatically. Manual discovery / zone refresh actions remain available, and the translated light-test action sends a short red validation color to the selected zone before starting screen sync.

## Validation mode

For automated smoke checks, the GUI supports:

```text
LUMAWAY_GUI_AUTOSTART=1
LUMAWAY_GUI_QUIT_AFTER_SYNC=1
LUMAWAY_GUI_ECHO_LOGS=1
LUMAWAY_GUI_DURATION_MS=5000
```

This opens the GUI, starts sync, mirrors logs to stdout, exits after the sync finishes, and leaves the normal Start/Stop code path under test.

## Launch

Open `LumaWay` from the application menu, or run:

```text
gio launch ~/.local/share/applications/io.github.BunnySweety.LumaWay.desktop
```

The Portal selector should appear in the graphical session. After selection, logs stay visible in the application window.
When a classified bridge, Portal, or capture error occurs, the main window shows the translated explanation plus a contextual `Retry` and/or `Open Settings` action below Start.
The main window also exposes an `About` dialog with the app version, MPL-2.0 license, GitHub project links, local-processing privacy note, no-telemetry statement, and nominative Philips Hue compatibility note.
While the Portal selector is open, the main window shows `Choose the screen or window to sync`. If the desktop portal returns a `restore_token`, `lumaway sync` stores it in `LUMAWAY_PORTAL_RESTORE_TOKEN` and reuses it on the next sync; desktops that do not expose one keep showing the selector reminder each session.
If the selected Portal stream stops delivering frames for more than 5 seconds, `lumaway sync` exits with a classified Portal-stream error; the GUI returns to Start and exposes `Retry`.
If the Hue bridge becomes unreachable during active streaming, the next DTLS send error is annotated as bridge loss; `lumaway sync` exits, attempts to stop capture and deactivate Entertainment, and the GUI shows a translated bridge-lost recovery message.
After the computer resumes from sleep, `lumaway sync` detects the wall-clock / monotonic-clock gap, exits instead of silently reusing an expired Portal or DTLS session, and the GUI shows a translated sleep-resume recovery message.

## Optional local overrides

The wrapper reads these optional values from `~/.config/lumaway/lumaway.env`:

```text
LUMAWAY_BIN=/custom/path/to/lumaway
LUMAWAY_PROFILE=default
LUMAWAY_SYNC_MODE=video
LUMAWAY_DURATION_MS=0
LUMAWAY_BRIGHTNESS=1.00
LUMAWAY_REACTIVITY=0.35
LUMAWAY_PORTAL_RESTORE_TOKEN=
LUMAWAY_AUTOSTART_SYNC=false
LUMAWAY_HUE_PIN_CERTS=1
LUMAWAY_HUE_PIN_MODE=spki
```

With pinning enabled, the first successful HTTPS session to the bridge stores a leaf **SPKI** hash; `LUMAWAY_BRIDGE_ID` selects the `by-id/` pin file once the hardware id is known. Use `LUMAWAY_HUE_PIN_MODE=cert` only for legacy full-certificate pins.

For entertainment **DTLS** (UDP 2100), `LUMAWAY_BRIDGE` must be a private or link-local address unless you set `LUMAWAY_DTLS_ALLOW_REMOTE=1`. `lumaway doctor` checks `LUMAWAY_CLIENT_KEY` and this target.

Keep `LUMAWAY_DURATION_MS=0` for normal long-running use. CLI commands load this file automatically, then load `LUMAWAY_PROFILE` non-secret capture/color defaults from `~/.config/lumaway/profiles/<name>.env`; the GUI-saved file is the source of truth for commands run without explicit flags. `LUMAWAY_SYNC_MODE=video|game|desktop` chooses the screen mode and resolves the matching preset (`video-wayland`, `game-wayland`, `desktop-wayland`). `LUMAWAY_PRESET=tv-wayland` remains accepted as a legacy alias for Video when no sync mode is set. `LUMAWAY_BRIGHTNESS` and `LUMAWAY_REACTIVITY` are written by the GUI sliders as values from `0.00` to `1.00`. `LUMAWAY_COLOR_PROFILE` is kept as an advanced compatibility value; when `LUMAWAY_SYNC_MODE` is set, the mode default wins. `LUMAWAY_PORTAL_RESTORE_TOKEN` is managed by `lumaway sync` when the desktop portal supports persistent ScreenCast selections. `LUMAWAY_AUTOSTART_SYNC=true` starts sync automatically when the application opens.

Settings has two separate startup options. `Open LumaWay when you sign in` creates or removes the XDG autostart entry at `~/.config/autostart/io.github.BunnySweety.LumaWay.desktop`. `Start sync when app opens` writes `LUMAWAY_AUTOSTART_SYNC=true` and starts screen sync after the app opens.

The Settings window includes a `Capture profile` field. Enter `default` to use `~/.config/lumaway/profiles/default.env`, or another profile name after creating it with `lumaway profile-template --name <name>`. Use `lumaway profile-list`, or the `Profiles` button in Settings, to list existing profile names. The `Quality` button runs `lumaway capture-quality --portal --sync-mode video --preset video-wayland --frames 30` with the saved settings and prints a compact capture summary plus a recommended next action in the log. The `Calibrate` button runs `lumaway calibrate-capture --name <profile> --force`, writes the measured profile, and saves `LUMAWAY_PROFILE` for future sync runs. If the bridge rejects the saved Hue application key, the GUI marks pairing as required and tells the user to press the bridge button before using `Pair` in Settings. Successful pairing immediately saves the replacement `LUMAWAY_APP_KEY` and `LUMAWAY_CLIENT_KEY` before loading zones. Loading zones also fetches the bridge name and writes `LUMAWAY_BRIDGE_ID` when the API returns it.

CLI equivalents: `lumaway auth --bridge <ip>` and `lumaway bridge-info --bridge <ip> --app-key <key>` update `lumaway.env` the same way. `lumaway doctor --bridge <ip> --app-key <key>` reports pinning status and saves the bridge id when credentials work.
