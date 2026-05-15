# Open Questions


**Plan produit** : [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

## DTLS

- `openssl::ssl::SslConnector` over a connected UDP socket completed the Hue DTLS-PSK handshake successfully against bridge `192.168.1.108`, area `TV`, on 2026-05-10.
- Can Hue Entertainment be implemented without a long-lived subprocess?
- What recovery behavior is needed after a failed DTLS handshake?
- Are `PSK-AES128-GCM-SHA256:PSK-CHACHA20-POLY1305` sufficient across supported Hue Bridge firmware versions?
- Repeated HueStream RGB frames at 25 FPS changed the `TV` area to red successfully.

## Portals and Capture

- `ashpd` is sufficient for the initial ScreenCast selection flow.
  - Validated through `lumaway portal-probe` in GNOME Wayland as user `bunny`.
  - Returned PipeWire node `107`, size `2560x1440`, position `(2560, 0)`.
- Should GStreamer target PipeWire node ID initially and later upgrade to `pipewire-serial` when available?
  - Initial portal capture works with `pipewiresrc fd=<portal fd> path=<node id>`.
  - Validated 65 frames in 2029 ms, about 32.03 FPS, in GNOME Wayland.
- Which GStreamer plugins are required in the Flatpak runtime?

## Secrets

- Use `oo7`, libsecret bindings, or another Secret Service integration?
- What fallback is acceptable for development builds only?

## Config

- JSON or TOML for user-visible config?
- How should config migrations be versioned?

## Product

- Final icon direction.
- Short application description.
- Whether the first public release should include only CLI or also GTK.
