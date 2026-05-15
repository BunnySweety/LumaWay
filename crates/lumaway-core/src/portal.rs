use ashpd::desktop::{
    screencast::{CursorMode, Screencast, SourceType},
    PersistMode,
};
use std::os::fd::OwnedFd;

use crate::{CoreError, Result};

#[derive(Debug, Clone)]
pub struct PortalStreamInfo {
    pub pipewire_node_id: u32,
    pub size: Option<(i32, i32)>,
    pub position: Option<(i32, i32)>,
}

#[derive(Debug)]
pub struct PortalSelection {
    pub stream: PortalStreamInfo,
    pub pipewire_fd: OwnedFd,
}

pub struct PortalScreenCast;

impl PortalScreenCast {
    pub async fn select_streams() -> Result<Vec<PortalStreamInfo>> {
        Ok(Self::select()
            .await?
            .into_iter()
            .map(|s| s.stream)
            .collect())
    }

    pub async fn select() -> Result<Vec<PortalSelection>> {
        let proxy = Screencast::new()
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;
        let session = proxy
            .create_session()
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;

        proxy
            .select_sources(
                &session,
                CursorMode::Metadata,
                SourceType::Monitor | SourceType::Window,
                false,
                None,
                PersistMode::DoNot,
            )
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;

        let response = proxy
            .start(&session, None)
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?
            .response()
            .map_err(|err| CoreError::Portal(err.to_string()))?;

        let fd = proxy
            .open_pipe_wire_remote(&session)
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;

        let streams: Vec<_> = response
            .streams()
            .iter()
            .map(|stream| PortalStreamInfo {
                pipewire_node_id: stream.pipe_wire_node_id(),
                size: stream.size(),
                position: stream.position(),
            })
            .collect();

        streams
            .into_iter()
            .map(|stream| {
                fd.try_clone()
                    .map(|pipewire_fd| PortalSelection {
                        stream,
                        pipewire_fd,
                    })
                    .map_err(|err| CoreError::Portal(err.to_string()))
            })
            .collect()
    }
}
