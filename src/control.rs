//! Generic "toggle control" abstraction shared by every Teams action.
//!
//! Each OpenDeck action this plugin exposes — mute and camera today; hand-raise
//! and blur planned — is structurally the same *toggle control*: pressing the
//! key publishes a fixed JSON command to teams-for-linux over MQTT, and the key
//! reflects a piece of live Teams state. Each control owns its full
//! state → display mapping (titles + images), so the generic runtime
//! ([`MqttController`], the display pusher) never branches on a specific
//! control. Adding a control is a small `impl` plus a registry entry in
//! `action::register_controls`.
//!
//! [`MqttController`]: crate::mqtt::MqttController

use crate::state::TeamsState;

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
    fn display(state: TeamsState, configured: bool) -> Display;

    /// Whether a key press should publish [`COMMAND`](Self::COMMAND) right now.
    fn can_activate(state: TeamsState, configured: bool) -> bool;
}

/// In a call, configured, and connected — the precondition every control shares
/// before it will act on a press.
fn ready(state: TeamsState, configured: bool) -> bool {
    configured && state.in_active_call()
}

/// Microphone mute/unmute.
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

    fn display(state: TeamsState, configured: bool) -> Display {
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

    fn can_activate(state: TeamsState, configured: bool) -> bool {
        ready(state, configured)
    }
}

/// Camera on/off (Teams "toggle video").
pub struct CameraControl;

static CAM_ON: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/cam@2x.png")));
static CAM_OFF: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/cam-off@2x.png")));
static CAM_DISABLED: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/cam-disabled@2x.png")));

impl Control for CameraControl {
    const UUID: &'static str = "com.geoffdavis.teamsforlinux.toggle-camera";
    const COMMAND: &'static str = r#"{"action":"toggle-video"}"#;

    fn display(state: TeamsState, configured: bool) -> Display {
        if !configured {
            Display {
                state_index: 0,
                title: "SETUP",
                image: CAM_DISABLED.as_str(),
            }
        } else if !state.in_active_call() {
            Display {
                state_index: 0,
                title: "OFF",
                image: CAM_DISABLED.as_str(),
            }
        } else if state.camera_on == Some(true) {
            Display {
                state_index: 0,
                title: "CAM",
                image: CAM_ON.as_str(),
            }
        } else {
            // In a call but camera off (or not yet reported).
            Display {
                state_index: 1,
                title: "CAM OFF",
                image: CAM_OFF.as_str(),
            }
        }
    }

    fn can_activate(state: TeamsState, configured: bool) -> bool {
        ready(state, configured)
    }
}

/// Raise/lower hand (Teams "toggle-hand-raise").
///
/// A one-way toggle: teams-for-linux accepts the command but publishes no
/// hand-raise status, so — unlike mute/camera — the key cannot reflect whether
/// the hand is currently raised. It only shows call/configured state (active in
/// a call, disabled otherwise) and fires the toggle on press.
pub struct HandControl;

static HAND_ON: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/hand@2x.png")));
static HAND_DISABLED: LazyLock<String> =
    LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/hand-disabled@2x.png")));

impl Control for HandControl {
    const UUID: &'static str = "com.geoffdavis.teamsforlinux.toggle-hand-raise";
    const COMMAND: &'static str = r#"{"action":"toggle-hand-raise"}"#;

    fn display(state: TeamsState, configured: bool) -> Display {
        // No raised/lowered state to mirror, so there is a single active look;
        // index 0 always (the manifest action has one state).
        if !configured {
            Display {
                state_index: 0,
                title: "SETUP",
                image: HAND_DISABLED.as_str(),
            }
        } else if !state.in_active_call() {
            Display {
                state_index: 0,
                title: "OFF",
                image: HAND_DISABLED.as_str(),
            }
        } else {
            Display {
                state_index: 0,
                title: "HAND",
                image: HAND_ON.as_str(),
            }
        }
    }

    fn can_activate(state: TeamsState, configured: bool) -> bool {
        ready(state, configured)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_call(muted: bool) -> TeamsState {
        TeamsState {
            muted,
            in_call: Some(true),
            camera_on: None,
        }
    }

    // -- mute --

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
        let d = MuteControl::display(TeamsState::default(), false);
        assert_eq!(d.title, "SETUP");
    }

    #[test]
    fn mute_display_off_when_not_in_call() {
        // Unknown call state (None) and an ended call (Some(false)) both show OFF.
        for in_call in [None, Some(false)] {
            let d = MuteControl::display(
                TeamsState {
                    in_call,
                    ..Default::default()
                },
                true,
            );
            assert_eq!(d.title, "OFF", "in_call = {in_call:?}");
            assert_eq!(d.image, MUTE_OFF.as_str());
        }
    }

    #[test]
    fn mute_display_mic_and_muted() {
        assert_eq!(MuteControl::display(in_call(false), true).title, "MIC");
        let muted = MuteControl::display(in_call(true), true);
        assert_eq!(muted.title, "MUTED");
        assert_eq!(muted.state_index, 1);
        assert_eq!(muted.image, MUTE_MUTED.as_str());
    }

    // -- camera --

    #[test]
    fn camera_control_command_and_uuid() {
        assert_eq!(
            CameraControl::UUID,
            "com.geoffdavis.teamsforlinux.toggle-camera"
        );
        assert_eq!(CameraControl::COMMAND, r#"{"action":"toggle-video"}"#);
    }

    #[test]
    fn camera_display_setup_and_off() {
        assert_eq!(
            CameraControl::display(TeamsState::default(), false).title,
            "SETUP"
        );
        // Configured but not in a call → OFF (disabled image).
        let off = CameraControl::display(TeamsState::default(), true);
        assert_eq!(off.title, "OFF");
        assert_eq!(off.image, CAM_DISABLED.as_str());
    }

    #[test]
    fn camera_display_on_when_camera_on_in_call() {
        let mut s = in_call(false);
        s.camera_on = Some(true);
        let d = CameraControl::display(s, true);
        assert_eq!(d.title, "CAM");
        assert_eq!(d.state_index, 0);
        assert_eq!(d.image, CAM_ON.as_str());
    }

    #[test]
    fn camera_display_off_when_camera_off_or_unknown_in_call() {
        for cam in [Some(false), None] {
            let mut s = in_call(false);
            s.camera_on = cam;
            let d = CameraControl::display(s, true);
            assert_eq!(d.title, "CAM OFF", "camera_on = {cam:?}");
            assert_eq!(d.state_index, 1);
            assert_eq!(d.image, CAM_OFF.as_str());
        }
    }

    // -- hand-raise --

    #[test]
    fn hand_control_command_and_uuid() {
        assert_eq!(
            HandControl::UUID,
            "com.geoffdavis.teamsforlinux.toggle-hand-raise"
        );
        assert_eq!(HandControl::COMMAND, r#"{"action":"toggle-hand-raise"}"#);
    }

    #[test]
    fn hand_display_setup_off_and_active() {
        // Unconfigured and configured-but-not-in-call both show the disabled
        // image; only an active call shows the lit hand.
        let setup = HandControl::display(TeamsState::default(), false);
        assert_eq!(setup.title, "SETUP");
        assert_eq!(setup.image, HAND_DISABLED.as_str());
        let off = HandControl::display(TeamsState::default(), true);
        assert_eq!(off.title, "OFF");
        assert_eq!(off.image, HAND_DISABLED.as_str());

        let on = HandControl::display(in_call(false), true);
        assert_eq!(on.title, "HAND");
        assert_eq!(on.state_index, 0);
        assert_eq!(on.image, HAND_ON.as_str());
    }

    // -- shared activation guard --

    #[test]
    fn controls_activate_only_in_call_and_configured() {
        for active in [
            MuteControl::can_activate(in_call(false), true),
            CameraControl::can_activate(in_call(false), true),
            HandControl::can_activate(in_call(false), true),
        ] {
            assert!(active);
        }
        assert!(!MuteControl::can_activate(in_call(false), false));
        assert!(!CameraControl::can_activate(TeamsState::default(), true));
        assert!(!HandControl::can_activate(TeamsState::default(), true));
    }

    #[test]
    fn action_images_are_distinct_data_uris() {
        for uri in [
            MUTE_NORMAL.as_str(),
            CAM_ON.as_str(),
            CAM_OFF.as_str(),
            CAM_DISABLED.as_str(),
            HAND_ON.as_str(),
            HAND_DISABLED.as_str(),
        ] {
            assert!(uri.starts_with("data:image/png;base64,"));
        }
        assert_ne!(CAM_ON.as_str(), CAM_OFF.as_str());
        assert_ne!(CAM_ON.as_str(), MUTE_NORMAL.as_str());
        assert_ne!(HAND_ON.as_str(), HAND_DISABLED.as_str());
        assert_ne!(HAND_ON.as_str(), CAM_ON.as_str());
    }
}
