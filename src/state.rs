//! Pure Teams media state machine (microphone, camera, call) driven by MQTT.

/// Which subscribed MQTT topic a payload arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTopic {
    Microphone,
    MicrophoneControl,
    Camera,
    InCall,
}

/// Teams media/call state as derived from MQTT. Each control reads the fields
/// it cares about; `None` options mean "no message seen yet".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TeamsState {
    pub muted: bool,
    pub camera_on: Option<bool>,
    pub in_call: Option<bool>,
}

impl TeamsState {
    pub fn in_active_call(&self) -> bool {
        self.in_call == Some(true)
    }

    /// Apply an MQTT payload. Returns true if the state changed.
    #[must_use = "returns false if the payload caused no state change"]
    pub fn apply(&mut self, topic: StateTopic, payload: &str) -> bool {
        let before = *self;
        match topic {
            StateTopic::MicrophoneControl => match payload {
                "muted" => self.muted = true,
                "unmuted" => self.muted = false,
                "off" => self.end_call(),
                _ => {}
            },
            StateTopic::Microphone => match payload {
                "true" | "muted" => self.muted = true,
                "false" | "speaking" | "silent" => self.muted = false,
                "off" => self.end_call(),
                _ => {}
            },
            StateTopic::Camera => match payload {
                "true" => self.camera_on = Some(true),
                "false" => self.camera_on = Some(false),
                _ => {}
            },
            StateTopic::InCall => match payload {
                "true" => self.in_call = Some(true),
                "false" => self.end_call(),
                _ => {}
            },
        }
        *self != before
    }

    /// The call ended: reset transient media state. Camera goes back to
    /// unknown since its retained value no longer reflects a live call.
    fn end_call(&mut self) {
        self.in_call = Some(false);
        self.muted = false;
        self.camera_on = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(muted: bool, in_call: Option<bool>) -> TeamsState {
        TeamsState {
            muted,
            in_call,
            camera_on: None,
        }
    }

    // -- microphone/control topic --

    #[test]
    fn control_muted_sets_muted() {
        let mut s = TeamsState::default();
        assert!(s.apply(StateTopic::MicrophoneControl, "muted"));
        assert_eq!(s, state(true, None));
    }

    #[test]
    fn control_unmuted_clears_muted() {
        let mut s = state(true, Some(true));
        assert!(s.apply(StateTopic::MicrophoneControl, "unmuted"));
        assert_eq!(s, state(false, Some(true)));
    }

    #[test]
    fn control_off_ends_call_and_unmutes() {
        let mut s = state(true, Some(true));
        assert!(s.apply(StateTopic::MicrophoneControl, "off"));
        assert_eq!(s, state(false, Some(false)));
    }

    #[test]
    fn control_unknown_payload_ignored() {
        let mut s = state(true, Some(true));
        assert!(!s.apply(StateTopic::MicrophoneControl, "weird"));
        assert_eq!(s, state(true, Some(true)));
    }

    // -- microphone topic (rich + boolean payloads) --

    #[test]
    fn microphone_true_and_muted_set_muted() {
        for payload in ["true", "muted"] {
            let mut s = state(false, Some(true));
            assert!(s.apply(StateTopic::Microphone, payload));
            assert_eq!(s, state(true, Some(true)), "payload {payload}");
        }
    }

    #[test]
    fn microphone_false_speaking_silent_clear_muted() {
        for payload in ["false", "speaking", "silent"] {
            let mut s = state(true, Some(true));
            assert!(s.apply(StateTopic::Microphone, payload));
            assert_eq!(s, state(false, Some(true)), "payload {payload}");
        }
    }

    #[test]
    fn microphone_off_ends_call_and_unmutes() {
        let mut s = state(true, Some(true));
        assert!(s.apply(StateTopic::Microphone, "off"));
        assert_eq!(s, state(false, Some(false)));
    }

    #[test]
    fn microphone_unknown_payload_ignored() {
        let mut s = state(false, Some(true));
        assert!(!s.apply(StateTopic::Microphone, "garbled"));
        assert_eq!(s, state(false, Some(true)));
    }

    // -- camera topic --

    #[test]
    fn camera_true_false_set_state() {
        let mut s = state(false, Some(true));
        assert!(s.apply(StateTopic::Camera, "true"));
        assert_eq!(s.camera_on, Some(true));
        assert!(s.apply(StateTopic::Camera, "false"));
        assert_eq!(s.camera_on, Some(false));
    }

    #[test]
    fn camera_unknown_payload_ignored() {
        let mut s = state(false, Some(true));
        assert!(!s.apply(StateTopic::Camera, "maybe"));
        assert_eq!(s.camera_on, None);
    }

    #[test]
    fn ending_call_clears_camera() {
        let mut s = state(false, Some(true));
        assert!(s.apply(StateTopic::Camera, "true"));
        assert!(s.apply(StateTopic::InCall, "false"));
        assert_eq!(s.camera_on, None);
        assert_eq!(s.in_call, Some(false));
    }

    // -- in-call topic --

    #[test]
    fn in_call_true_sets_active() {
        let mut s = TeamsState::default();
        assert!(s.apply(StateTopic::InCall, "true"));
        assert_eq!(s, state(false, Some(true)));
    }

    #[test]
    fn in_call_false_resets_mute() {
        let mut s = state(true, Some(true));
        assert!(s.apply(StateTopic::InCall, "false"));
        assert_eq!(s, state(false, Some(false)));
    }

    #[test]
    fn apply_returns_false_when_nothing_changes() {
        let mut s = state(false, Some(true));
        assert!(!s.apply(StateTopic::InCall, "true"));
    }
}
