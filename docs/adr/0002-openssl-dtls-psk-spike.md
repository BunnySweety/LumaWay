# ADR 0002: OpenSSL DTLS-PSK Spike

## Status

Accepted for the first implementation.

## Context

Hue Entertainment uses DTLS-PSK over UDP port 2100. LumaWay needs a native Rust path that avoids using a long-lived `openssl s_client` subprocess as the production transport.

The current goal is to validate whether the Rust `openssl` crate can complete the Hue DTLS-PSK handshake and send HueStream frames reliably.

## Decision

Add an isolated `DtlsTransport` trait and an OpenSSL-backed `DtlsHueTransport` implementation.

The implementation uses:

- `SslMethod::dtls_client()`;
- `set_psk_client_callback`;
- `hue-application-id` as the PSK identity;
- `client_key` decoded from hex as the PSK;
- a connected UDP socket wrapped with `Read` and `Write`;
- cipher list `PSK-AES128-GCM-SHA256:PSK-CHACHA20-POLY1305`.

## Consequences

- The rest of the Hue client remains independent of OpenSSL.
- The CLI can now reach the real DTLS boundary with `lumaway test-color`.
- Hardware validation succeeded against bridge `192.168.1.108`, area `TV`, on 2026-05-10.
- The first implementation can proceed with this OpenSSL-backed transport while keeping the `DtlsTransport` abstraction.

## Reconsider If

- OpenSSL over connected UDP does not complete Hue handshakes reliably.
- Hue requires OpenSSL options not exposed safely enough by the crate.
- DTLS send semantics need retransmission or timing control beyond `SslStream`.
- A maintained Rust DTLS implementation with PSK support becomes clearly better.
