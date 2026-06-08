//! Generic "toggle control" abstraction shared by every Teams action.
//!
//! Each OpenDeck action this plugin exposes — mute today; camera, hand-raise
//! and blur planned — is structurally the same *toggle control*: pressing the
//! key publishes a fixed JSON command to teams-for-linux over MQTT, and the
//! key reflects a piece of live Teams state. Each control owns its full
//! state → display mapping (titles + images), so the generic runtime
//! ([`MqttController`], the display pusher) never branches on a specific
//! control. Adding a control is a small `impl` plus a registry entry in
//! `action::register_controls`.
//!
//! [`MqttController`]: crate::mqtt::MqttController

use crate::state::MicState;

use base64::Engine as _;
use std::sync::LazyLock;

fn png_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// What a control's key should show: which manifest state to select, its title,
/// and the key image as a `data:` URI. Produced per-control by
/// [`Control::display`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
    pub state_index: u16,
    pub title: &'static str,
    pub image: &'static str,
}

/// A Teams toggle control bound to exactly one OpenDeck action UUID.
///
/// Implementors are zero-sized marker types; everything they contribute is
/// constants and pure functions, which keeps the runtime free of per-control
/// branching.
pub trait Control: Send + Sync + 'static {
    /// Manifest action UUID (`Actions[].UUID` in `plugin/manifest.json`).
    const UUID: &'static str;
    /// JSON payload published to the command topic on a key press.
    const COMMAND: &'static str;

    /// Map the current Teams state to what the key should show, including the
    /// resolved image for this control.
    fn display(state: MicState, configured: bool) -> Display;

    /// Whether a key press should publish [`COMMAND`](Self::COMMAND) right now.
    fn can_activate(state: MicState, configured: bool) -> bool;
}

/// Microphone mute/unmute — the original (and currently only) control.
pub struct MuteControl;

static MUTE_NORMAL: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon@2x.png")));
static MUTE_MUTED: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-muted@2x.png")));
static MUTE_OFF: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-off@2x.png")));

impl Control for MuteControl {
    const UUID: &'static str = "com.geoffdavis.teamsforlinux.toggle-mute";
    const COMMAND: &'static str = r#"{"action":"toggle-mute"}"#;

    fn display(state: MicState, configured: bool) -> Display {
        // SETUP: not configured yet. OFF: not in a call (presses are ignored).
        // Otherwise reflect the live mute state on state index 1 (muted) or 0.
        if !configured {
            Display {
                state_index: 0,
                title: "SETUP",
                image: MUTE_OFF.as_str(),
            }
        } else if !state.in_active_call() {
            Display {
                state_index: 0,
                title: "OFF",
                image: MUTE_OFF.as_str(),
            }
        } else if state.muted {
            Display {
                state_index: 1,
                title: "MUTED",
                image: MUTE_MUTED.as_str(),
            }
        } else {
            Display {
                state_index: 0,
                title: "MIC",
                image: MUTE_NORMAL.as_str(),
            }
        }
    }

    fn can_activate(state: MicState, configured: bool) -> bool {
        configured && state.in_active_call()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_call(muted: bool) -> MicState {
        MicState {
            muted,
            in_call: Some(true),
        }
    }

    #[test]
    fn mute_control_command_and_uuid() {
        assert_eq!(
            MuteControl::UUID,
            "com.geoffdavis.teamsforlinux.toggle-mute"
        );
        assert_eq!(MuteControl::COMMAND, r#"{"action":"toggle-mute"}"#);
    }

    #[test]
    fn mute_display_setup_when_unconfigured() {
        let d = MuteControl::display(MicState::default(), false);
        assert_eq!(d.title, "SETUP");
        assert_eq!(d.state_index, 0);
    }

    #[test]
    fn mute_display_off_when_not_in_call_even_if_muted() {
        let d = MuteControl::display(
            MicState {
                muted: true,
                in_call: Some(false),
            },
            true,
        );
        assert_eq!(d.title, "OFF");
        assert_eq!(d.state_index, 0);
        assert_eq!(d.image, MUTE_OFF.as_str());
    }

    #[test]
    fn mute_display_mic_when_in_call_unmuted() {
        let d = MuteControl::display(in_call(false), true);
        assert_eq!(d.title, "MIC");
        assert_eq!(d.state_index, 0);
        assert_eq!(d.image, MUTE_NORMAL.as_str());
    }

    #[test]
    fn mute_display_muted_when_in_call_muted() {
        let d = MuteControl::display(in_call(true), true);
        assert_eq!(d.title, "MUTED");
        assert_eq!(d.state_index, 1);
        assert_eq!(d.image, MUTE_MUTED.as_str());
    }

    #[test]
    fn mute_display_images_are_data_uris_and_distinct() {
        let normal = MuteControl::display(in_call(false), true).image;
        let muted = MuteControl::display(in_call(true), true).image;
        assert!(normal.starts_with("data:image/png;base64,"));
        assert_ne!(normal, muted);
    }

    #[test]
    fn mute_control_can_activate_only_in_call_and_configured() {
        assert!(MuteControl::can_activate(in_call(false), true));
        assert!(!MuteControl::can_activate(in_call(false), false));
        assert!(!MuteControl::can_activate(MicState::default(), true));
    }
}
