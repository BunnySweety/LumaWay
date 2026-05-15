# Security model (LumaWay)

This document summarizes trust boundaries and known limitations for operators and contributors. It is not a formal threat model audit.

Product roadmap (separate from security hardening): [plan-hue-sync-daily.md](plan-hue-sync-daily.md).

## Hue bridge HTTPS (`lumaway-hue`)

Philips Hue bridges expose HTTPS with a **self-signed certificate**.

- **Default:** the HTTP client uses **relaxed TLS certificate verification** (`danger_accept_invalid_certs`) so pairing and API calls succeed without importing a custom trust anchor.
- **Optional pinning:** set **`LUMAWAY_HUE_PIN_CERTS=1`** (also accepts `true`, `yes`, `on`). The client uses rustls with a custom verifier. On the **first** successful TLS handshake it stores a **32-byte SHA-256** under **`hue-tls-pins/`** in the Lumaway config directory (default **`$XDG_CONFIG_HOME/lumaway`**, or **`$HOME/.config/lumaway`**; override the root with **`LUMAWAY_HUE_PIN_DIR`**). Later connections must match that pin or TLS fails. Handshake signatures are still verified (ring / webpki).
- **Pin material (default):** **`LUMAWAY_HUE_PIN_MODE=spki`** (default) hashes the leaf **SubjectPublicKeyInfo** → file **`<bridge-ip>.spki.sha256`**. Survives leaf certificate re-issue with the same key. Legacy **`LUMAWAY_HUE_PIN_MODE=cert`** hashes the full leaf DER → **`<bridge-ip>.sha256`** (still accepted as a fallback lookup when using SPKI mode).
- **Bridge id binding:** **`LUMAWAY_BRIDGE_ID`** is written to `~/.config/lumaway/lumaway.env` after `lumaway auth`, `lumaway bridge-info`, `lumaway doctor` (with credentials), or GUI pairing / zone load. It selects **`hue-tls-pins/by-id/<id>.spki.sha256`** and triggers TLS pin promotion when pinning is enabled.

**Implication:** on a hostile or shared LAN, a machine-in-the-middle could impersonate the bridge IP you configured and intercept or alter REST traffic (including during pairing, or on the **first** connection when learning a pin). Mitigations are operational: use a trusted network, verify bridge identity out-of-band, reserve a DHCP lease for the bridge, and prefer wired Ethernet where practical. If the bridge **rotates its key** (SPKI changes) or you switch pin mode, delete the relevant `*.sha256` files and reconnect once with pinning enabled to re-learn.

### Operator checklist (HTTPS)

- Prefer a **known-good bridge IP** (static DHCP lease or reserved address) so configuration does not silently follow a rogue host.
- After discovery, compare the bridge **hardware / app identity** with what the API reports (e.g. bridge id in responses / Hue app) when possible.
- Treat the LAN as a **trust zone**: guest Wi‑Fi, untrusted VLANs, or shared flats increase MITM risk for both HTTPS and DTLS.

### TLS pinning details

- **Environment:** `LUMAWAY_HUE_PIN_CERTS` enables pinning; `LUMAWAY_HUE_PIN_MODE` is `spki` (default) or `cert`; `LUMAWAY_BRIDGE_ID` optional hardware id; `LUMAWAY_HUE_PIN_DIR` replaces the Lumaway config root.
- **Lookup order (SPKI mode):** `by-id/<id>.spki.sha256` → `<ip>.spki.sha256` → legacy `<ip>.sha256` (cert pin, backward compatible).
- **File permissions (Unix):** pin files are chmod **0o600**.

## Hue entertainment DTLS (UDP port 2100)

The entertainment stream uses **DTLS with a pre-shared key** derived from the Hue pairing (`client_key`). OpenSSL is configured for **PSK ciphers** with **no X.509 certificate verification** (`SslVerifyMode::NONE`), which matches how Hue exposes this channel: there is no meaningful bridge leaf cert to validate in the same way as HTTPS.

**Implication:** secrecy and integrity of the stream rely on the **PSK** and on traffic reaching the **correct host** at the configured IP. The same LAN MITM considerations as for HTTPS apply if an attacker can redirect UDP to another host that speaks the Hue protocol.

**Operational checks (implemented):**

- Before opening DTLS, LumaWay validates that `LUMAWAY_BRIDGE` is a **private IPv4** or **link-local / unique-local IPv6** address (Hue bridges are LAN devices). Set **`LUMAWAY_DTLS_ALLOW_REMOTE=1`** only if you deliberately target a non-LAN address.
- `lumaway doctor` reports whether `LUMAWAY_CLIENT_KEY` is set and whether the bridge IP passes this check.
- PSK identity overrides: `LUMAWAY_DTLS_IDENTITY`, `LUMAWAY_DTLS_USE_APP_KEY` (see rustdoc on `resolve_dtls_psk_identity`).
- At `info` log level (`RUST_LOG=lumaway=info`), sync logs the configured bridge IP, resolved UDP socket address, and handshake completion (PSK lengths only, not secrets).

## Discovery and network noise

- **Cloud discovery** (`discovery.meethue.com`) resolves bridges with outbound HTTPS; it reveals that the client is looking for Hue hardware.
- **SSDP** and **local TCP scan** discover bridges on the LAN. Subnet scanning uses a **bounded worker pool**; concurrency can be tuned with `LUMAWAY_SUBNET_SCAN_CONCURRENCY` (default 64, clamped 1–256).

## Desktop GUI (`lumaway-gui`)

- **`LUMAWAY_BIN`** must be an **absolute** path to a **regular file** that is **not world-writable**. Invalid values are ignored and the GUI falls back to the usual `lumaway` resolution order.
- Secrets live under `~/.config/lumaway/`; the app sets restrictive file permissions where applicable.

## Reporting

For security-sensitive bugs, use the repository’s private reporting channel if one is configured; otherwise open a confidential issue per GitHub’s guidance for the maintainer.
