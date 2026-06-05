//! Settings: property-inspector values layered over an optional config file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Raw per-instance settings as stored by OpenDeck (all strings; empty = inherit).
/// Field names are the contract with `plugin/pi/index.html`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PiSettings {
    pub broker_host: String,
    pub broker_port: String,
    pub username: String,
    pub password: String,
    pub topic_prefix: String,
    pub microphone_topic: String,
    pub microphone_control_topic: String,
    pub in_call_topic: String,
    pub command_topic: String,
}

impl PiSettings {
    /// True if the user has set anything at all in the property inspector.
    pub fn any_set(&self) -> bool {
        [
            &self.broker_host,
            &self.broker_port,
            &self.username,
            &self.password,
            &self.topic_prefix,
            &self.microphone_topic,
            &self.microphone_control_topic,
            &self.in_call_topic,
            &self.command_topic,
        ]
        .iter()
        .any(|f| !f.trim().is_empty())
    }
}

/// Optional declarative config file (TOML), e.g. written by the home-manager module.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default)]
pub struct FileSettings {
    pub broker_host: Option<String>,
    pub broker_port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<String>,
    /// Path to a file whose (trimmed) contents become the password. Wins over `password`.
    pub password_file: Option<String>,
    pub topic_prefix: Option<String>,
    pub microphone_topic: Option<String>,
    pub microphone_control_topic: Option<String>,
    pub in_call_topic: Option<String>,
    pub command_topic: Option<String>,
}

/// Fully resolved, ready-to-use settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub broker_host: String,
    pub broker_port: u16,
    pub username: String,
    pub password: String,
    pub topic_prefix: String,
    pub microphone_suffix: String,
    pub microphone_control_suffix: String,
    pub in_call_suffix: String,
    pub command_suffix: String,
    pub configured: bool,
}

impl Resolved {
    pub fn microphone_topic(&self) -> String {
        join_topic(&self.topic_prefix, &self.microphone_suffix)
    }
    pub fn microphone_control_topic(&self) -> String {
        join_topic(&self.topic_prefix, &self.microphone_control_suffix)
    }
    pub fn in_call_topic(&self) -> String {
        join_topic(&self.topic_prefix, &self.in_call_suffix)
    }
    pub fn command_topic(&self) -> String {
        join_topic(&self.topic_prefix, &self.command_suffix)
    }
}

/// `prefix/suffix` with stray slashes trimmed from both parts.
pub fn join_topic(prefix: &str, suffix: &str) -> String {
    format!("{}/{}", prefix.trim_matches('/'), suffix.trim_matches('/'))
}

/// Default path: `$XDG_CONFIG_HOME/opendeck-teams-for-linux/config.toml`
/// (falling back to `~/.config`).
pub fn config_file_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .filter(|home| !home.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/"))
                .join(".config")
        });
    base.join("opendeck-teams-for-linux").join("config.toml")
}

/// Load and parse the config file. Returns None if it is missing or invalid
/// (invalid files are logged and treated as absent). Resolves `password_file`.
pub fn load_file_settings(path: &Path) -> Option<FileSettings> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut file: FileSettings = match toml::from_str(&text) {
        Ok(parsed) => parsed,
        Err(err) => {
            log::warn!("ignoring invalid config file: {err}");
            return None;
        }
    };
    if let Some(pw_path) = &file.password_file {
        match std::fs::read_to_string(pw_path) {
            Ok(pw) => {
                // trim_end only: leading whitespace in a password is intentional.
                file.password = Some(pw.trim_end().to_string());
            }
            Err(err) => log::warn!("failed to read password_file ({pw_path}): {err}"),
        }
    }
    Some(file)
}

/// Layer: defaults < file < non-empty PI fields.
pub fn resolve(file: Option<&FileSettings>, pi: &PiSettings) -> Resolved {
    let f = file.cloned().unwrap_or_default();
    let pick = |pi_value: &str, file_value: &Option<String>, default: &str| -> String {
        let pi_value = pi_value.trim();
        if !pi_value.is_empty() {
            pi_value.to_string()
        } else if let Some(v) = file_value {
            v.trim().to_string()
        } else {
            default.to_string()
        }
    };
    let pi_port = pi.broker_port.trim().parse::<u16>().ok();
    Resolved {
        broker_host: pick(&pi.broker_host, &f.broker_host, "127.0.0.1"),
        broker_port: pi_port.or(f.broker_port).unwrap_or(1883),
        username: pick(&pi.username, &f.username, ""),
        password: pick(&pi.password, &f.password, ""),
        topic_prefix: pick(&pi.topic_prefix, &f.topic_prefix, "teams"),
        microphone_suffix: pick(&pi.microphone_topic, &f.microphone_topic, "microphone"),
        microphone_control_suffix: pick(
            &pi.microphone_control_topic,
            &f.microphone_control_topic,
            "microphone/control",
        ),
        in_call_suffix: pick(&pi.in_call_topic, &f.in_call_topic, "in-call"),
        command_suffix: pick(&pi.command_topic, &f.command_topic, "command"),
        configured: file.is_some() || pi.any_set(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn any_set_false_for_default() {
        assert!(!PiSettings::default().any_set());
    }

    #[test]
    fn any_set_ignores_whitespace_but_sees_values() {
        let pi_ws = PiSettings {
            username: "  ".into(),
            ..Default::default()
        };
        assert!(!pi_ws.any_set());
        let pi_val = PiSettings {
            username: "geoff".into(),
            ..Default::default()
        };
        assert!(pi_val.any_set());
    }

    #[test]
    fn join_topic_trims_slashes() {
        assert_eq!(join_topic("teams", "microphone"), "teams/microphone");
        assert_eq!(
            join_topic("/teams/", "/microphone/control/"),
            "teams/microphone/control"
        );
    }

    #[test]
    fn resolve_pure_defaults_unconfigured() {
        let r = resolve(None, &PiSettings::default());
        assert!(!r.configured);
        assert_eq!(r.broker_host, "127.0.0.1");
        assert_eq!(r.broker_port, 1883);
        assert_eq!(r.username, "");
        assert_eq!(r.password, "");
        assert_eq!(r.microphone_topic(), "teams/microphone");
        assert_eq!(r.microphone_control_topic(), "teams/microphone/control");
        assert_eq!(r.in_call_topic(), "teams/in-call");
        assert_eq!(r.command_topic(), "teams/command");
    }

    #[test]
    fn resolve_file_overrides_defaults_and_marks_configured() {
        let file = FileSettings {
            broker_host: Some("10.0.0.5".into()),
            broker_port: Some(1884),
            topic_prefix: Some("work-teams".into()),
            ..Default::default()
        };
        let r = resolve(Some(&file), &PiSettings::default());
        assert!(r.configured);
        assert_eq!(r.broker_host, "10.0.0.5");
        assert_eq!(r.broker_port, 1884);
        assert_eq!(r.in_call_topic(), "work-teams/in-call");
    }

    #[test]
    fn resolve_pi_overrides_file() {
        let file = FileSettings {
            broker_host: Some("10.0.0.5".into()),
            username: Some("file-user".into()),
            ..Default::default()
        };
        let pi = PiSettings {
            username: "pi-user".into(),
            ..Default::default()
        };
        let r = resolve(Some(&file), &pi);
        assert_eq!(r.broker_host, "10.0.0.5"); // empty PI field inherits
        assert_eq!(r.username, "pi-user"); // non-empty PI field wins
    }

    #[test]
    fn resolve_pi_only_marks_configured() {
        let pi = PiSettings {
            broker_host: "127.0.0.1".into(),
            ..Default::default()
        };
        let r = resolve(None, &pi);
        assert!(r.configured);
    }

    #[test]
    fn resolve_invalid_pi_port_falls_through() {
        let file = FileSettings {
            broker_port: Some(2000),
            ..Default::default()
        };
        let pi = PiSettings {
            broker_port: "not-a-port".into(),
            ..Default::default()
        };
        let r = resolve(Some(&file), &pi);
        assert_eq!(r.broker_port, 2000);
    }

    #[test]
    fn load_missing_file_is_none() {
        assert!(load_file_settings(Path::new("/nonexistent/config.toml")).is_none());
    }

    #[test]
    fn load_invalid_toml_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is { not toml").unwrap();
        assert!(load_file_settings(&path).is_none());
    }

    #[test]
    fn load_parses_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "username = \"geoff\"\nbroker_port = 1884\n").unwrap();
        let fs = load_file_settings(&path).unwrap();
        assert_eq!(fs.username.as_deref(), Some("geoff"));
        assert_eq!(fs.broker_port, Some(1884));
    }

    #[test]
    fn load_password_file_wins_over_password() {
        let dir = tempfile::tempdir().unwrap();
        let pw_path = dir.path().join("password");
        let mut pw = std::fs::File::create(&pw_path).unwrap();
        writeln!(pw, "s3cret").unwrap(); // trailing newline must be trimmed
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            format!(
                "password = \"inline\"\npassword_file = \"{}\"\n",
                pw_path.display()
            ),
        )
        .unwrap();
        let fs = load_file_settings(&path).unwrap();
        assert_eq!(fs.password.as_deref(), Some("s3cret"));
    }

    #[test]
    fn load_unreadable_password_file_keeps_inline_password() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            "password = \"inline\"\npassword_file = \"/nonexistent/pw\"\n",
        )
        .unwrap();
        let fs = load_file_settings(&path).unwrap();
        assert_eq!(fs.password.as_deref(), Some("inline"));
    }
}
