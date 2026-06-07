//! Generic "toggle control" abstraction shared by every Teams action.
//!
//! Each OpenDeck action this plugin exposes — mute today; camera, hand-raise
//! and blur planned — is structurally the same *toggle control*: pressing the
//! key publishes a fixed JSON command to teams-for-linux over MQTT, and the
//! key reflects a piece of live Teams state. The parts that differ per control
//! live behind the [`Control`] trait, so adding a control is a small `impl`
//! plus a registry entry in `action::register_controls`.
//!
//! Generalising the *state* itself (a per-control state type rather than the
//! shared [`MicState`]) is tracked separately and intentionally out of scope
//! here — every control currently reads the same mic/in-call state machine.

use crate::state::{self, Display, Icon, MicState};

use base64::Engine as _;
use std::sync::LazyLock;

fn png_data_uri(bytes: &[u8]) -> String {
    format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// A Teams toggle control bound to exactly one OpenDeck action UUID.
///
/// Implementors are zero-sized marker types; everything they contribute is
/// constants and pure functions, which keeps the runtime ([`MqttController`],
/// the display pusher) free of per-control branching.
///
/// [`MqttController`]: crate::mqtt::MqttController
pub trait Control: Send + Sync + 'static {
    /// Manifest action UUID (`Actions[].UUID` in `plugin/manifest.json`).
    const UUID: &'static str;
    /// JSON payload published to the command topic on a key press.
    const COMMAND: &'static str;

    /// Map the current Teams state to what the key should show.
    fn display(state: MicState, configured: bool) -> Display;

    /// Whether a key press should publish [`COMMAND`](Self::COMMAND) right now.
    fn can_activate(state: MicState, configured: bool) -> bool;

    /// Resolve a display [`Icon`] to a `data:` URI for one of this control's
    /// embedded images.
    fn icon_data_uri(icon: Icon) -> &'static str;
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
        state::display(state, configured)
    }

    fn can_activate(state: MicState, configured: bool) -> bool {
        state::can_toggle(state, configured)
    }

    fn icon_data_uri(icon: Icon) -> &'static str {
        match icon {
            Icon::Normal => &MUTE_NORMAL,
            Icon::Muted => &MUTE_MUTED,
            Icon::Off => &MUTE_OFF,
        }
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
    fn mute_control_display_delegates_to_state() {
        assert_eq!(
            MuteControl::display(in_call(true), true),
            state::display(in_call(true), true)
        );
        assert_eq!(MuteControl::display(in_call(true), true).title, "MUTED");
    }

    #[test]
    fn mute_control_can_activate_only_in_call_and_configured() {
        assert!(MuteControl::can_activate(in_call(false), true));
        assert!(!MuteControl::can_activate(in_call(false), false));
        assert!(!MuteControl::can_activate(MicState::default(), true));
    }

    #[test]
    fn mute_control_resolves_each_icon() {
        for icon in [Icon::Normal, Icon::Muted, Icon::Off] {
            assert!(MuteControl::icon_data_uri(icon).starts_with("data:image/png;base64,"));
        }
        // Distinct images per state.
        assert_ne!(
            MuteControl::icon_data_uri(Icon::Normal),
            MuteControl::icon_data_uri(Icon::Muted)
        );
    }
}
