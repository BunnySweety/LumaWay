# ADR 0001: Rust, GTK4/libadwaita, Flatpak, Linux Wayland

## Status

Accepted.

## Context

LumaWay is a native Hue Sync application for Linux Wayland. The core product depends on screen capture through XDG Desktop Portal, PipeWire/GStreamer frame processing, Hue REST API calls, Hue Entertainment DTLS streaming, and a native Linux desktop interface.

The project is intentionally not targeting Windows, macOS, X11-native operation, mobile, web, or cloud sync.

## Decision

Use:

- Rust for the application core and CLI;
- GTK4/libadwaita for the desktop UI;
- Flatpak as the primary distribution format;
- Linux Wayland as the platform boundary;
- a headless engine before the GUI.

## Alternatives Considered

- Python + PyGObject: faster to prototype, but weaker for long-term runtime control and packaging.
- Go: strong for networking and CLI work, but less natural for GTK/GStreamer/PipeWire integration and likely to require substantial CGO.
- Tauri: useful for web UI desktop apps, but the main complexity here is native capture, multimedia, and DTLS rather than UI rendering.
- Qt/PySide: mature, but less aligned with GNOME/libadwaita and Flatpak-first Linux Wayland integration.

## Consequences

- Early work must validate the risky native pieces before UI polish.
- The first milestone is a CLI that sends a fixed color to a Hue Entertainment Area.
- GTK UI work is intentionally delayed until Hue DTLS and capture/sync are proven.
- The project can stay focused and avoid cross-platform abstractions that do not serve Linux Wayland.

## Reconsider If

- Rust DTLS proves impractical without unacceptable unsafe or subprocess-heavy code.
- GStreamer Rust bindings block reliable Wayland portal capture.
- GTK4/libadwaita creates unacceptable packaging or runtime issues in Flatpak.
