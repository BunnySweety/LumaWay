use crate::i18n;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserMessageCode {
    ZoneOff,
    MissingSyncConfig,
    HueLinkButtonNotPressed,
    HueAuthRejected,
    HueBridgeUnavailable,
    HueBridgeLost,
    HueDtlsFailed,
    HueAreaConflict,
    PortalCancelled,
    PortalUnavailable,
    PortalStreamClosed,
    CaptureTimeout,
    CaptureTooDark,
    SystemSleepResume,
    Unknown,
}

impl UserMessageCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ZoneOff => "gui.zone_off",
            Self::MissingSyncConfig => "gui.missing_sync_config",
            Self::HueLinkButtonNotPressed => "hue.link_button_not_pressed",
            Self::HueAuthRejected => "hue.auth_rejected",
            Self::HueBridgeUnavailable => "hue.bridge_unavailable",
            Self::HueBridgeLost => "hue.bridge_lost",
            Self::HueDtlsFailed => "hue.dtls_failed",
            Self::HueAreaConflict => "hue.area_conflict",
            Self::PortalCancelled => "portal.cancelled",
            Self::PortalUnavailable => "portal.unavailable",
            Self::PortalStreamClosed => "portal.stream_closed",
            Self::CaptureTimeout => "capture.timeout",
            Self::CaptureTooDark => "capture.too_dark",
            Self::SystemSleepResume => "system.sleep_resume",
            Self::Unknown => "unknown",
        }
    }

    pub fn summary(self) -> String {
        match self {
            Self::ZoneOff => {
                i18n::tr("Turn on the selected Entertainment zone before starting sync.")
            }
            Self::MissingSyncConfig => i18n::tr(
                "Bridge address, zone, and pairing keys are required before sync can start.",
            ),
            Self::HueLinkButtonNotPressed => {
                i18n::tr("The Hue bridge button was not pressed before pairing.")
            }
            Self::HueAuthRejected => i18n::tr("The Hue bridge rejected the saved pairing key."),
            Self::HueBridgeUnavailable => i18n::tr("The Hue bridge is unreachable."),
            Self::HueBridgeLost => i18n::tr("The Hue bridge connection was lost during sync."),
            Self::HueDtlsFailed => i18n::tr("The Hue Entertainment connection failed."),
            Self::HueAreaConflict => {
                i18n::tr("Another Hue app may already be using this Entertainment zone.")
            }
            Self::PortalCancelled => i18n::tr("Screen selection was cancelled."),
            Self::PortalUnavailable => {
                i18n::tr("Screen capture is not available through the desktop portal.")
            }
            Self::PortalStreamClosed => i18n::tr("The screen capture stream stopped."),
            Self::CaptureTimeout => i18n::tr("No screen frames arrived from the selected source."),
            Self::CaptureTooDark => i18n::tr("Screen capture is too dark to sync reliably."),
            Self::SystemSleepResume => {
                i18n::tr("LumaWay stopped sync after the computer resumed from sleep.")
            }
            Self::Unknown => i18n::tr("Something went wrong."),
        }
    }

    pub fn action(self) -> String {
        match self {
            Self::ZoneOff => i18n::tr("Enable the zone switch, then start sync again."),
            Self::MissingSyncConfig => {
                i18n::tr("Open Settings, pair the bridge, and select an Entertainment zone.")
            }
            Self::HueLinkButtonNotPressed => {
                i18n::tr("Press the physical button on the bridge, then press Pair again.")
            }
            Self::HueAuthRejected => {
                i18n::tr("Press the bridge button, then use Pair in Settings to create new keys.")
            }
            Self::HueBridgeUnavailable => {
                i18n::tr("Check that the bridge is powered on and reachable on the network.")
            }
            Self::HueBridgeLost => i18n::tr(
                "Check that the bridge is powered on and reachable, then start sync again.",
            ),
            Self::HueDtlsFailed => {
                i18n::tr("Stop sync, wait a few seconds, then start sync again.")
            }
            Self::HueAreaConflict => {
                i18n::tr("Stop the other Hue sync app or choose a different Entertainment zone.")
            }
            Self::PortalCancelled => {
                i18n::tr("Start sync again and choose the screen or window to sync.")
            }
            Self::PortalUnavailable => i18n::tr(
                "Check that xdg-desktop-portal and the desktop portal backend are running.",
            ),
            Self::PortalStreamClosed => {
                i18n::tr("Start sync again and choose the screen or window to sync.")
            }
            Self::CaptureTimeout => {
                i18n::tr("Start sync again and reselect the screen in the portal dialog.")
            }
            Self::CaptureTooDark => {
                i18n::tr("Run backend probe, then use Quality or Calibrate if needed.")
            }
            Self::SystemSleepResume => {
                i18n::tr("Start sync again and reselect the screen if the portal asks.")
            }
            Self::Unknown => i18n::tr("Check the details below, then try again."),
        }
    }

    pub fn is_bridge_error(self) -> bool {
        matches!(
            self,
            Self::HueAuthRejected
                | Self::HueLinkButtonNotPressed
                | Self::HueBridgeUnavailable
                | Self::HueBridgeLost
                | Self::HueDtlsFailed
                | Self::HueAreaConflict
        )
    }

    pub fn is_portal_or_capture_error(self) -> bool {
        matches!(
            self,
            Self::PortalCancelled
                | Self::PortalUnavailable
                | Self::PortalStreamClosed
                | Self::CaptureTimeout
                | Self::CaptureTooDark
        )
    }

    pub fn offers_retry_action(self) -> bool {
        matches!(
            self,
            Self::ZoneOff
                | Self::HueBridgeUnavailable
                | Self::HueBridgeLost
                | Self::HueDtlsFailed
                | Self::HueAreaConflict
                | Self::PortalCancelled
                | Self::PortalUnavailable
                | Self::PortalStreamClosed
                | Self::CaptureTimeout
                | Self::SystemSleepResume
        )
    }

    pub fn offers_settings_action(self) -> bool {
        matches!(
            self,
            Self::MissingSyncConfig
                | Self::HueLinkButtonNotPressed
                | Self::HueAuthRejected
                | Self::HueBridgeUnavailable
                | Self::HueBridgeLost
                | Self::HueAreaConflict
                | Self::PortalUnavailable
                | Self::CaptureTooDark
        )
    }
}

pub fn classify_error(error: &str) -> UserMessageCode {
    let normalized = error.trim();
    let lower = normalized.to_lowercase();

    if lower.contains("zone is off") {
        return UserMessageCode::ZoneOff;
    }
    if lower.contains("bridge address, zone, and keys are required")
        || lower.contains("zone, bridge address, or application key missing")
        || lower.contains("bridge address and application key are required")
    {
        return UserMessageCode::MissingSyncConfig;
    }
    if lower.contains("link button not pressed")
        || lower.contains("bridge button was not pressed")
        || lower.contains("press the bridge button")
    {
        return UserMessageCode::HueLinkButtonNotPressed;
    }
    if lower.contains("hue bridge authentication failed")
        || lower.contains("saved hue application key was rejected")
        || lower.contains("unauthorized user")
    {
        return UserMessageCode::HueAuthRejected;
    }
    if lower.contains("entertainment area")
        && (lower.contains("already")
            || lower.contains("active")
            || lower.contains("in use")
            || lower.contains("conflict"))
    {
        return UserMessageCode::HueAreaConflict;
    }
    if lower.contains("bridge lost during sync")
        || lower.contains("hue bridge connection was lost during sync")
    {
        return UserMessageCode::HueBridgeLost;
    }
    if lower.contains("hue entertainment dtls failed")
        || lower.contains("dtls connect attempt failed")
        || lower.contains("dtls handshake")
    {
        return UserMessageCode::HueDtlsFailed;
    }
    if lower.contains("hue request failed")
        || lower.contains("connection refused")
        || lower.contains("connection timed out")
        || lower.contains("no route to host")
        || lower.contains("network is unreachable")
    {
        return UserMessageCode::HueBridgeUnavailable;
    }
    if lower.contains("portal returned no streams")
        || (lower.contains("portal") && (lower.contains("cancel") || lower.contains("denied")))
    {
        return UserMessageCode::PortalCancelled;
    }
    if lower.contains("portal error")
        || lower.contains("org.freedesktop.portal")
        || lower.contains("screencast interface")
        || lower.contains("xdg-desktop-portal")
    {
        return UserMessageCode::PortalUnavailable;
    }
    if lower.contains("capture timed out waiting for a frame") {
        return UserMessageCode::CaptureTimeout;
    }
    if lower.contains("portal stream stopped") || lower.contains("screen capture stream stopped") {
        return UserMessageCode::PortalStreamClosed;
    }
    if lower.contains("system sleep detected")
        || lower.contains("computer resumed from sleep")
        || lower.contains("sync stopped after resume")
    {
        return UserMessageCode::SystemSleepResume;
    }
    if lower.contains("capture_too_dark")
        || lower.contains("dark frames")
        || lower.contains("returns black")
    {
        return UserMessageCode::CaptureTooDark;
    }

    UserMessageCode::Unknown
}

pub fn format_user_error(error: &str) -> String {
    let code = classify_error(error);
    let mut text = format!(
        "error[{}]: {}\n{}\n",
        code.as_str(),
        code.summary(),
        code.action()
    );
    if code == UserMessageCode::Unknown {
        let details = normalize_details(error);
        if !details.is_empty() {
            text.push_str(&i18n::tr_format(
                "Details: {details}",
                &[("details", &details)],
            ));
            text.push('\n');
        }
    }
    text
}

fn normalize_details(error: &str) -> String {
    error
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{classify_error, UserMessageCode};

    #[test]
    fn classifies_hue_authentication_errors() {
        assert_eq!(
            classify_error("Hue bridge rejected the request: link button not pressed"),
            UserMessageCode::HueLinkButtonNotPressed
        );
        assert_eq!(
            classify_error("Error: Hue bridge authentication failed"),
            UserMessageCode::HueAuthRejected
        );
        assert_eq!(
            classify_error("saved Hue application key was rejected; pair again"),
            UserMessageCode::HueAuthRejected
        );
    }

    #[test]
    fn classifies_portal_and_capture_errors() {
        assert_eq!(
            classify_error("Error: portal returned no streams"),
            UserMessageCode::PortalCancelled
        );
        assert_eq!(
            classify_error("portal error: org.freedesktop.portal.ScreenCast unavailable"),
            UserMessageCode::PortalUnavailable
        );
        assert_eq!(
            classify_error("capture timed out waiting for a frame"),
            UserMessageCode::CaptureTimeout
        );
        assert_eq!(
            classify_error("portal stream stopped: no screen frames arrived for 5 seconds"),
            UserMessageCode::PortalStreamClosed
        );
        assert_eq!(
            classify_error("system sleep detected: sync stopped after resume; start sync again"),
            UserMessageCode::SystemSleepResume
        );
    }

    #[test]
    fn classifies_bridge_and_entertainment_errors() {
        assert_eq!(
            classify_error("Hue request failed: connection timed out"),
            UserMessageCode::HueBridgeUnavailable
        );
        assert_eq!(
            classify_error("bridge lost during sync while sending Hue Entertainment frame: Hue Entertainment DTLS failed: connection timed out"),
            UserMessageCode::HueBridgeLost
        );
        assert_eq!(
            classify_error("Hue Entertainment DTLS failed: handshake failed"),
            UserMessageCode::HueDtlsFailed
        );
        assert_eq!(
            classify_error("entertainment area already active"),
            UserMessageCode::HueAreaConflict
        );
    }

    #[test]
    fn keeps_unknown_errors_generic() {
        assert_eq!(
            classify_error("unexpected parser error"),
            UserMessageCode::Unknown
        );
    }

    #[test]
    fn offers_contextual_recovery_actions() {
        assert!(UserMessageCode::PortalCancelled.offers_retry_action());
        assert!(!UserMessageCode::PortalCancelled.offers_settings_action());
        assert!(!UserMessageCode::HueLinkButtonNotPressed.offers_retry_action());
        assert!(UserMessageCode::HueLinkButtonNotPressed.offers_settings_action());
        assert!(UserMessageCode::HueAuthRejected.offers_settings_action());
        assert!(!UserMessageCode::HueAuthRejected.offers_retry_action());
        assert!(UserMessageCode::HueBridgeUnavailable.offers_retry_action());
        assert!(UserMessageCode::HueBridgeUnavailable.offers_settings_action());
        assert!(UserMessageCode::HueBridgeLost.offers_retry_action());
        assert!(UserMessageCode::HueBridgeLost.offers_settings_action());
        assert!(UserMessageCode::PortalStreamClosed.offers_retry_action());
        assert!(!UserMessageCode::PortalStreamClosed.offers_settings_action());
        assert!(UserMessageCode::SystemSleepResume.offers_retry_action());
        assert!(!UserMessageCode::SystemSleepResume.offers_settings_action());
        assert!(!UserMessageCode::Unknown.offers_retry_action());
        assert!(!UserMessageCode::Unknown.offers_settings_action());
    }
}
