//! Routes parsed CLI `Command` variants to async handlers.

use anyhow::{bail, Result};
use lumaway_core::{GStreamerTestCapture, PortalScreenCast, SyncMode};
use lumaway_hue::{
    discover_bridges, resolve_dtls_psk_identity, ChannelColor, DtlsHueTransport, HueBridgeConfig,
    HueClient, HueColor,
};
use std::time::Duration;
use tracing::info;

use crate::bridge_env::persist_bridge_identity;
use crate::doctor;
use crate::ColorProfile;
use crate::Command;
use crate::{
    ensure_profile_loaded, list_profiles, resolve_preset_for_mode, run_backend_probe,
    run_calibrate_capture, run_capture_quality, run_detect_crop, run_sample_bench,
    run_sample_debug, run_sync, run_sync_bench, send_fixed_color, write_profile_template,
};

pub async fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::DiscoverBridges => {
            ensure_profile_loaded()?;
            let bridges = discover_bridges().await?;
            println!("{}", serde_json::to_string_pretty(&bridges)?);
            if !bridges.is_empty() {
                eprintln!(
                    "hint: after pairing, run `lumaway bridge-info --bridge <ip> --app-key <key>` to save LUMAWAY_BRIDGE_ID in lumaway.env"
                );
            }
            Ok(())
        }
        Command::ProfileList => list_profiles(),
        Command::ProfileTemplate { name, force } => write_profile_template(&name, force),
        Command::CalibrateCapture {
            name,
            frames,
            sample_width,
            sample_height,
            fps,
            dark_threshold,
            force,
        } => {
            run_calibrate_capture(
                &name,
                frames,
                sample_width,
                sample_height,
                fps,
                dark_threshold,
                force,
            )
            .await
        }
        Command::Auth { bridge } => {
            ensure_profile_loaded()?;
            info!(%bridge, "auth command requested");
            let client = HueClient::new(HueBridgeConfig::new(bridge.clone()))?;
            let user = client.create_user("lumaway").await?;
            if !user.app_key.trim().is_empty() {
                let authed = HueClient::new(HueBridgeConfig {
                    bridge_ip: bridge.clone(),
                    app_key: Some(user.app_key.clone()),
                    client_key: user.client_key.clone(),
                })?;
                if let Ok(info) = authed.bridge_info().await {
                    persist_bridge_identity(&bridge, &info.id)?;
                }
            }
            println!("{}", serde_json::to_string_pretty(&user)?);
            Ok(())
        }
        Command::ListAreas { bridge, app_key } => {
            ensure_profile_loaded()?;
            info!(%bridge, "list-areas command requested");
            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge,
                app_key: Some(app_key),
                client_key: None,
            })?;
            let areas = client.entertainment_areas_with_light_counts().await?;
            println!("{}", serde_json::to_string_pretty(&areas)?);
            Ok(())
        }
        Command::BridgeInfo { bridge, app_key } => {
            ensure_profile_loaded()?;
            info!(%bridge, "bridge-info command requested");
            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge.clone(),
                app_key: Some(app_key),
                client_key: None,
            })?;
            let info = client.bridge_info().await?;
            persist_bridge_identity(&bridge, &info.id)?;
            println!("{}", serde_json::to_string_pretty(&info)?);
            Ok(())
        }
        Command::ActivateArea {
            bridge,
            app_key,
            area,
            brightness,
        } => {
            ensure_profile_loaded()?;
            if !(1.0..=100.0).contains(&brightness) {
                bail!("brightness must be between 1 and 100");
            }
            info!(%bridge, %area, brightness, "activate-area command requested");
            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge,
                app_key: Some(app_key),
                client_key: None,
            })?;
            let entertainment_area = client.resolve_entertainment_area(&area).await?;
            let lights = client
                .set_entertainment_area_lights(&area, true, Some(brightness))
                .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": entertainment_area.id,
                    "name": entertainment_area.name,
                    "active": true,
                    "lights": lights
                }))?
            );
            Ok(())
        }
        Command::DeactivateArea {
            bridge,
            app_key,
            area,
        } => {
            ensure_profile_loaded()?;
            info!(%bridge, %area, "deactivate-area command requested");
            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge,
                app_key: Some(app_key),
                client_key: None,
            })?;
            let entertainment_area = client.resolve_entertainment_area(&area).await?;
            let lights = client
                .set_entertainment_area_lights(&area, false, None)
                .await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "id": entertainment_area.id,
                    "name": entertainment_area.name,
                    "active": false,
                    "lights": lights
                }))?
            );
            Ok(())
        }
        Command::TestColor {
            bridge,
            app_key,
            client_key,
            area,
            color,
            duration_ms,
            fps,
        } => {
            ensure_profile_loaded()?;
            let parsed_color = HueColor::parse(&color)
                .ok_or_else(|| anyhow::anyhow!("unsupported color: {color}"))?;
            info!(%bridge, %area, ?parsed_color, duration_ms, fps, "test-color command requested");
            if fps == 0 {
                bail!("fps must be greater than zero");
            }

            let client = HueClient::new(HueBridgeConfig {
                bridge_ip: bridge.clone(),
                app_key: Some(app_key.clone()),
                client_key: Some(client_key.clone()),
            })?;
            let dtls_identity = resolve_dtls_psk_identity(&client, app_key.as_str()).await?;
            let entertainment_area = client.resolve_entertainment_area(&area).await?;
            let channels: Vec<ChannelColor> = entertainment_area
                .channels
                .into_iter()
                .map(|channel_id| ChannelColor {
                    channel_id: channel_id.channel_id,
                    color: parsed_color,
                })
                .collect();

            client
                .activate_entertainment(&entertainment_area.id)
                .await?;
            // Same ~500 ms pause as Lumux between REST activate and DTLS (bridge / streaming readiness).
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut transport = DtlsHueTransport::connect(&bridge, dtls_identity, client_key)?;
            let send_result = send_fixed_color(
                &mut transport,
                &entertainment_area.id,
                channels,
                duration_ms,
                fps,
            )
            .await;
            let deactivate_result = client
                .deactivate_entertainment(&entertainment_area.id)
                .await;

            match (send_result, deactivate_result) {
                (Ok(frames_sent), Ok(())) => {
                    info!(frames_sent, "sent HueStream RGB frames");
                    Ok(())
                }
                (Err(send_err), Ok(())) => Err(send_err),
                (Ok(_), Err(deactivate_err)) => Err(deactivate_err.into()),
                (Err(send_err), Err(deactivate_err)) => {
                    Err(send_err.context(format!("also failed to deactivate: {deactivate_err}")))
                }
            }
        }
        Command::CaptureStats {
            duration_ms,
            portal,
            width,
            height,
            fps,
        } => {
            ensure_profile_loaded()?;
            let capture = if portal {
                let mut selections = PortalScreenCast::select().await?;
                let selection = selections
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("portal returned no streams"))?;
                info!(
                    node_id = selection.stream.pipewire_node_id,
                    size = ?selection.stream.size,
                    position = ?selection.stream.position,
                    "selected portal stream"
                );
                GStreamerTestCapture::from_pipewire_node(
                    selection.stream.pipewire_node_id,
                    selection.pipewire_fd,
                    width,
                    height,
                    fps,
                )?
            } else {
                GStreamerTestCapture::new(width, height, fps)?
            };
            let stats = capture.run_for(std::time::Duration::from_millis(duration_ms))?;
            println!(
                "frames={} duration_ms={} fps={:.2}",
                stats.frames,
                stats.duration.as_millis(),
                stats.fps
            );
            Ok(())
        }
        Command::SampleBench {
            portal,
            frames,
            bands,
            grids,
            fps,
        } => {
            ensure_profile_loaded()?;
            run_sample_bench(portal, frames, bands, &grids, fps).await
        }
        Command::SampleDebug {
            portal,
            bridge,
            app_key,
            area,
            sync_mode,
            preset,
            frames,
            fps,
            capture_fps,
            pipewire_fps,
            capture_backend,
            sample_width,
            sample_height,
            sample_edge_margin,
            sampling,
            sample_crop_left,
            sample_crop_right,
            sample_crop_top,
            sample_crop_bottom,
            smoothing,
            brightness,
            color_profile,
            noise_threshold,
            max_step,
        } => {
            ensure_profile_loaded()?;
            ensure_screen_mode(sync_mode)?;
            run_sample_debug(
                portal,
                bridge,
                app_key,
                area,
                resolve_preset_for_mode(sync_mode, preset),
                frames,
                fps,
                capture_fps,
                pipewire_fps,
                capture_backend,
                sample_width,
                sample_height,
                sample_edge_margin,
                sampling,
                sample_crop_left,
                sample_crop_right,
                sample_crop_top,
                sample_crop_bottom,
                smoothing,
                brightness,
                resolve_color_profile(sync_mode, color_profile),
                noise_threshold,
                max_step,
            )
            .await
        }
        Command::CaptureQuality {
            portal,
            bridge,
            app_key,
            area,
            sync_mode,
            preset,
            frames,
            fps,
            capture_fps,
            pipewire_fps,
            capture_backend,
            sample_width,
            sample_height,
            sample_edge_margin,
            sampling,
            sample_crop_left,
            sample_crop_right,
            sample_crop_top,
            sample_crop_bottom,
            color_profile,
        } => {
            ensure_profile_loaded()?;
            ensure_screen_mode(sync_mode)?;
            run_capture_quality(
                portal,
                bridge,
                app_key,
                area,
                resolve_preset_for_mode(sync_mode, preset),
                frames,
                fps,
                capture_fps,
                pipewire_fps,
                capture_backend,
                sample_width,
                sample_height,
                sample_edge_margin,
                sampling,
                sample_crop_left,
                sample_crop_right,
                sample_crop_top,
                sample_crop_bottom,
                resolve_color_profile(sync_mode, color_profile),
            )
            .await
        }
        Command::DetectCrop {
            portal,
            frames,
            sample_width,
            sample_height,
            fps,
            threshold,
            max_edge,
        } => {
            ensure_profile_loaded()?;
            run_detect_crop(
                portal,
                frames,
                sample_width,
                sample_height,
                fps,
                threshold,
                max_edge,
            )
            .await
        }
        Command::BackendProbe {
            frames,
            sample_width,
            sample_height,
            fps,
            dark_threshold,
        } => {
            ensure_profile_loaded()?;
            run_backend_probe(frames, sample_width, sample_height, fps, dark_threshold).await
        }
        Command::PortalProbe => {
            ensure_profile_loaded()?;
            let streams = PortalScreenCast::select_streams().await?;
            for stream in streams {
                println!(
                    "node_id={} size={:?} position={:?}",
                    stream.pipewire_node_id, stream.size, stream.position
                );
            }
            Ok(())
        }
        Command::Sync {
            bridge,
            app_key,
            client_key,
            area,
            sync_mode,
            preset,
            duration_ms,
            fps,
            capture_fps,
            stream_fps,
            pipewire_fps,
            capture_backend,
            capture_poll_ms,
            sample_width,
            sample_height,
            sample_edge_margin,
            sampling,
            sample_crop_left,
            sample_crop_right,
            sample_crop_top,
            sample_crop_bottom,
            auto_crop,
            auto_crop_frames,
            auto_crop_threshold,
            auto_crop_max_edge,
            smoothing,
            brightness,
            color_profile,
            noise_threshold,
            max_step,
        } => {
            ensure_profile_loaded()?;
            ensure_screen_mode(sync_mode)?;
            run_sync(
                bridge,
                app_key,
                client_key,
                area,
                resolve_preset_for_mode(sync_mode, preset),
                duration_ms,
                fps,
                capture_fps,
                stream_fps,
                pipewire_fps,
                capture_backend,
                capture_poll_ms,
                sample_width,
                sample_height,
                sample_edge_margin,
                sampling,
                sample_crop_left,
                sample_crop_right,
                sample_crop_top,
                sample_crop_bottom,
                auto_crop,
                auto_crop_frames,
                auto_crop_threshold,
                auto_crop_max_edge,
                smoothing,
                brightness,
                resolve_color_profile(sync_mode, color_profile),
                noise_threshold,
                max_step,
            )
            .await
        }
        Command::SyncBench {
            duration_ms,
            capture_fps,
            stream_fps,
            pipewire_fps,
            capture_backend,
            capture_poll_ms,
            sample_width,
            sample_height,
            sample_edge_margin,
            smoothing,
            brightness,
            color_profile,
            noise_threshold,
            max_step,
        } => {
            ensure_profile_loaded()?;
            run_sync_bench(
                duration_ms,
                capture_fps,
                stream_fps,
                pipewire_fps,
                capture_backend,
                &capture_poll_ms,
                sample_width,
                sample_height,
                sample_edge_margin,
                smoothing,
                brightness,
                color_profile,
                noise_threshold,
                max_step,
            )
            .await
        }
        Command::Doctor { bridge, app_key } => doctor::run_doctor(bridge, app_key).await,
    }
}

fn ensure_screen_mode(sync_mode: Option<SyncMode>) -> Result<()> {
    if sync_mode == Some(SyncMode::Music) {
        bail!("LUMAWAY_SYNC_MODE=music requires the future audio-sync command");
    }
    Ok(())
}

fn resolve_color_profile(
    sync_mode: Option<SyncMode>,
    color_profile: Option<ColorProfile>,
) -> ColorProfile {
    if let Some(mode) = sync_mode {
        return match mode {
            SyncMode::Video => ColorProfile::Vivid,
            SyncMode::Game => ColorProfile::Game,
            SyncMode::Desktop => ColorProfile::Desktop,
            SyncMode::Music => ColorProfile::Vivid,
        };
    }
    color_profile.unwrap_or(ColorProfile::Vivid)
}
