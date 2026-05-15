//! Benchmark-only Hue area, no-op DTLS transport, and stream send helper.

use anyhow::Result;
use lumaway_hue::{
    DtlsTransport, EntertainmentArea, EntertainmentChannel, EntertainmentChannelPosition, HueColor,
    HueStreamMessage,
};

pub fn send_dtls_frame(transport: &mut impl DtlsTransport, message: &[u8]) -> Result<()> {
    transport.send_bytes(message)?;
    transport.drain_incoming()?;
    Ok(())
}

pub fn black() -> HueColor {
    HueColor {
        red: 0,
        green: 0,
        blue: 0,
    }
}

#[derive(Debug, Default)]
pub struct NullTransport {
    pub sent_bytes: usize,
}

impl DtlsTransport for NullTransport {
    fn send(&mut self, message: &HueStreamMessage) -> lumaway_hue::Result<()> {
        self.send_bytes(message.as_bytes())
    }

    fn send_bytes(&mut self, bytes: &[u8]) -> lumaway_hue::Result<()> {
        self.sent_bytes += bytes.len();
        Ok(())
    }
}

pub fn synthetic_bench_area() -> EntertainmentArea {
    EntertainmentArea {
        id: "00000000-0000-0000-0000-000000000000".into(),
        name: "Bench".into(),
        channels: vec![
            bench_channel(0, -1.0, 0.0),
            bench_channel(1, -0.6, 0.0),
            bench_channel(2, -0.2, 0.0),
            bench_channel(3, 0.2, 0.0),
            bench_channel(4, 0.6, 0.0),
            bench_channel(5, 1.0, 0.0),
        ],
        lights: None,
    }
}

fn bench_channel(channel_id: u8, x: f64, y: f64) -> EntertainmentChannel {
    EntertainmentChannel {
        channel_id,
        position: Some(EntertainmentChannelPosition { x, y, z: 0.0 }),
    }
}
