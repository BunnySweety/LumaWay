//! Hue DTLS RGB streaming and wait helpers (Ctrl-C / SIGTERM).

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use lumaway_hue::{ChannelColor, DtlsTransport, HueStreamFrame};

use crate::frame_delay_for_fps;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepOutcome {
    Completed,
    Interrupted,
}

pub async fn sleep_until_or_interrupt(deadline: Instant) -> Result<SleepOutcome> {
    let now = Instant::now();
    if deadline <= now {
        return Ok(SleepOutcome::Completed);
    }

    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .context("failed to listen for SIGTERM")?;
        tokio::select! {
            () = tokio::time::sleep(deadline - now) => Ok(SleepOutcome::Completed),
            result = tokio::signal::ctrl_c() => {
                result.context("failed to listen for Ctrl-C")?;
                Ok(SleepOutcome::Interrupted)
            }
            _ = terminate.recv() => Ok(SleepOutcome::Interrupted),
        }
    }

    #[cfg(not(unix))]
    tokio::select! {
        () = tokio::time::sleep(deadline - now) => Ok(SleepOutcome::Completed),
        result = tokio::signal::ctrl_c() => {
            result.context("failed to listen for Ctrl-C")?;
            Ok(SleepOutcome::Interrupted)
        }
    }
}

pub async fn send_fixed_color(
    transport: &mut impl DtlsTransport,
    area: &str,
    channels: Vec<ChannelColor>,
    duration_ms: u64,
    fps: u8,
) -> Result<u64> {
    let frame_delay = frame_delay_for_fps(fps);
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut sequence = 0u8;
    let mut frames_sent = 0u64;

    while Instant::now() < deadline {
        let frame = HueStreamFrame {
            entertainment_config_id: area.to_string(),
            sequence,
            channels: channels.clone(),
        };
        let message = frame.encode_rgb()?;
        transport.send(&message)?;
        sequence = sequence.wrapping_add(1);
        frames_sent += 1;
        tokio::time::sleep(frame_delay).await;
    }

    Ok(frames_sent)
}
