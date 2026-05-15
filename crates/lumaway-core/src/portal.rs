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
    pub restore_token: Option<String>,
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
        Self::select_with_options(None, PersistMode::DoNot).await
    }

    pub async fn select_persistent(restore_token: Option<&str>) -> Result<Vec<PortalSelection>> {
        Self::select_with_options(restore_token, PersistMode::ExplicitlyRevoked).await
    }

    async fn select_with_options(
        restore_token: Option<&str>,
        persist_mode: PersistMode,
    ) -> Result<Vec<PortalSelection>> {
        let proxy = Screencast::new()
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;
        let session = proxy
            .create_session()
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;
        let restore_token = normalize_restore_token(restore_token);

        proxy
            .select_sources(
                &session,
                CursorMode::Metadata,
                SourceType::Monitor | SourceType::Window,
                false,
                restore_token.as_deref(),
                persist_mode,
            )
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?;

        let response = proxy
            .start(&session, None)
            .await
            .map_err(|err| CoreError::Portal(err.to_string()))?
            .response()
            .map_err(|err| CoreError::Portal(err.to_string()))?;
        let response_restore_token = normalize_restore_token(response.restore_token());

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
                        restore_token: response_restore_token.clone(),
                    })
                    .map_err(|err| CoreError::Portal(err.to_string()))
            })
            .collect()
    }
}

fn normalize_restore_token(token: Option<&str>) -> Option<String> {
    token
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::normalize_restore_token;

    #[test]
    fn normalizes_restore_token() {
        assert_eq!(
            normalize_restore_token(Some(" token ")).as_deref(),
            Some("token")
        );
        assert_eq!(normalize_restore_token(Some("   ")), None);
        assert_eq!(normalize_restore_token(None), None);
    }
}
