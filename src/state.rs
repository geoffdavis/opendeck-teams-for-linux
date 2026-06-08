//! Pure mute/in-call state machine.

/// Which subscribed MQTT topic a payload arrived on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTopic {
    Microphone,
    MicrophoneControl,
    InCall,
}

/// Teams microphone/call state as derived from MQTT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MicState {
    pub muted: bool,
    /// None = unknown (no in-call message seen yet).
    pub in_call: Option<bool>,
}

impl MicState {
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
                "off" => {
                    self.in_call = Some(false);
                    self.muted = false;
                }
                _ => {}
            },
            StateTopic::Microphone => match payload {
                "true" | "muted" => self.muted = true,
                "false" | "speaking" | "silent" => self.muted = false,
                "off" => {
                    self.in_call = Some(false);
                    self.muted = false;
                }
                _ => {}
            },
            StateTopic::InCall => match payload {
                "true" => self.in_call = Some(true),
                "false" => {
                    self.in_call = Some(false);
                    self.muted = false;
                }
                _ => {}
            },
        }
        *self != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(muted: bool, in_call: Option<bool>) -> MicState {
        MicState { muted, in_call }
    }

    // -- microphone/control topic --

    #[test]
    fn control_muted_sets_muted() {
        let mut s = MicState::default();
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

    // -- in-call topic --

    #[test]
    fn in_call_true_sets_active() {
        let mut s = MicState::default();
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
