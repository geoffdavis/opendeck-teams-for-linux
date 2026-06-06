# opendeck-teams-for-linux Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and publish the `opendeck-teams-for-linux` repository: a Rust OpenDeck plugin that shows live Teams mute state and toggles mute via teams-for-linux's MQTT integration, shipped as a `.sdPlugin` zip on GitHub Releases plus a Nix flake with a home-manager module.

**Architecture:** A single Rust binary built on the `openaction` SDK crate (v2.6, verified against its source: `Action` trait with `will_appear`/`will_disappear`/`key_down`/`did_receive_settings`/`property_inspector_did_appear`, `Instance::{set_state,set_title,set_image,show_alert,send_to_property_inspector}`, and public `visible_instances(uuid)`). A background `rumqttc` task owns the broker connection and publishes mic state over a `tokio::sync::watch` channel; a display-pusher task maps state to button state/title/icon for all visible instances. Settings resolve as defaults < `~/.config/opendeck-teams-for-linux/config.toml` < non-empty property-inspector fields. Pure state machine and settings resolution are TDD'd; the full binary is verified by a protocol-level integration test (mock OpenDeck WebSocket server + throwaway mosquitto) that runs on any machine and in CI — **no OpenDeck installation or Stream Deck hardware needed** (the dev laptop has neither; OpenDeck is not in nixpkgs). The real-hardware smoke test happens on the work machine post-publish.

**Tech Stack:** Rust (edition 2024), `openaction` 2.6, `rumqttc` 0.25, `tokio`, `serde`/`serde_json`, `toml` 1, `base64` 0.22, `simplelog`; vanilla-JS property inspector; imagemagick icon generation; GitHub Actions with `cargo-zigbuild` for musl static builds; Nix flake (devShell via direnv/nix-direnv, package, home-manager module).

**Verified upstream facts (do not re-litigate during implementation):**

- OpenDeck selects the plugin binary via manifest `CodePaths` keyed by the **host gnu target triple** (`x86_64-unknown-linux-gnu` / `aarch64-unknown-linux-gnu`), taking precedence over `CodePathLin` (`manifest.rs` + `mod.rs:171` in nekename/OpenDeck). Our musl-built binaries are *named* by those gnu keys — the key identifies the OpenDeck host build, not our libc.
- Per-action `PropertyInspectorPath` and `DisableAutomaticStates` are supported manifest keys. We set `DisableAutomaticStates: true` because button state is MQTT-driven, not press-driven.
- The PI page defines the standard global `connectElgatoStreamDeckSocket(inPort, inUUID, inRegisterEvent, inInfo, inActionInfo)`; OpenDeck's webserver injects a shim that calls it. Register event is `registerPropertyInspector`. PI talks to the same WebSocket port: `setSettings` out; `didReceiveSettings` / `sendToPropertyInspector` in.
- `openaction::run(args)` parses `-port`/`-pluginuuid`/`-registerevent`/`-info` flags itself.

**Out of scope for this plan (tracked in the spec):** TLS (post-v1 release), OpenDeck store listing (after teams-for-linux PR #2608 merges), and the consumer-repo migration (separate follow-up plan in that repo — see spec § Migration plan).

**Working directory for all tasks:** `~/src/opendeck-teams-for-linux` (repo already exists with the spec committed).

---

### Task 1: Scaffold — cargo project, flake devShell, direnv, license

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs` (stub)
- Create: `flake.nix`
- Create: `.envrc`
- Create: `.gitignore`
- Create: `LICENSE`

- [ ] **Step 1: Write `.gitignore`**

```gitignore
/target
/dist
/result
```

- [ ] **Step 2: Write `LICENSE`** (MIT, current year, author Geoff Davis)

```text
MIT License

Copyright (c) 2026 Geoff Davis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 3: Write `Cargo.toml`**

```toml
[package]
name = "opendeck-teams-for-linux"
version = "0.1.0"
edition = "2024"
license = "MIT"
description = "OpenDeck plugin: Microsoft Teams (teams-for-linux) mute button with MQTT-driven state"
repository = "https://github.com/geoffdavis/opendeck-teams-for-linux"

[dependencies]
openaction = "2.6"
# default-features off: drops rustls/aws-lc-sys (needs external cmake). Plain-TCP v1.
rumqttc = { version = "0.25", default-features = false }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "1"
base64 = "0.22"
log = "0.4"
simplelog = "0.12"

[dev-dependencies]
tempfile = "3"
tokio-tungstenite = "0.28"
futures-util = "0.3"

[profile.release]
strip = true
lto = true
```

- [ ] **Step 4: Write stub `src/main.rs`**

```rust
fn main() {
	println!("opendeck-teams-for-linux: not yet implemented");
}
```

- [ ] **Step 5: Write `flake.nix`** (devShell + formatter only; package output comes in Task 9)

```nix
{
  description = "OpenDeck plugin: Microsoft Teams (teams-for-linux) mute button with MQTT-driven state";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux"];
    eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
  in {
    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          clippy
          rustfmt
          rust-analyzer
          imagemagick
          zip
          jq
          mosquitto
        ];
      };
    });

    formatter = eachSystem (pkgs: pkgs.alejandra);
  };
}
```

- [ ] **Step 6: Write `.envrc`**

```bash
use flake
```

- [ ] **Step 7: Activate the dev shell and generate the lockfiles**

Run:
```bash
cd ~/src/opendeck-teams-for-linux
direnv allow
nix flake lock
cargo generate-lockfile
```
Expected: `flake.lock` and `Cargo.lock` created; no errors. (If direnv hasn't loaded yet in this shell, prefix cargo commands with `nix develop -c` — same for all later tasks.)

- [ ] **Step 8: Verify the stub builds and the flake evaluates**

Run: `cargo build && nix flake check`
Expected: clean build of the stub binary; flake check passes.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "chore: scaffold cargo project, flake devShell, direnv, MIT license"
```

---

### Task 2: Icons — generation script and committed PNGs

**Files:**
- Create: `scripts/generate-icons.sh`
- Create (generated): `plugin/icons/icon.png`, `icon@2x.png`, `icon-off.png`, `icon-off@2x.png`, `icon-muted.png`, `icon-muted@2x.png`

This ports the `drawIcon` imagemagick derivations from the consumer repo's `modules/teams-for-linux.nix` 1:1 (purple mic = active, grey = off, red slash overlay = muted).

- [ ] **Step 1: Write `scripts/generate-icons.sh`**

```bash
#!/usr/bin/env bash
# Regenerate the plugin button icons with imagemagick.
# Ported from the original nix drawIcon derivations.
set -euo pipefail
cd "$(dirname "$0")/../plugin/icons"

draw() { # size background output
	local s=$1 bg=$2 out=$3
	magick -size "${s}x${s}" "xc:${bg}" \
		-fill white -stroke white -strokewidth $((s / 36)) \
		-draw "roundrectangle $((s * 38 / 100)),$((s * 22 / 100)) \
		                      $((s * 62 / 100)),$((s * 58 / 100)) \
		                      $((s * 12 / 100)),$((s * 12 / 100))" \
		-fill none \
		-draw "arc $((s * 30 / 100)),$((s * 40 / 100)) \
		           $((s * 70 / 100)),$((s * 70 / 100)) 0,180" \
		-draw "line $((s * 50 / 100)),$((s * 65 / 100)) \
		            $((s * 50 / 100)),$((s * 80 / 100))" \
		-draw "line $((s * 38 / 100)),$((s * 80 / 100)) \
		            $((s * 62 / 100)),$((s * 80 / 100))" \
		"$out"
}

draw 72 "#4a3fcf" icon.png
draw 144 "#4a3fcf" icon@2x.png
draw 72 "#5a5a5a" icon-off.png
draw 144 "#5a5a5a" icon-off@2x.png

magick icon.png -fill "#ff4040" -stroke "#ff4040" -strokewidth 6 \
	-draw "line 14,58 58,14" icon-muted.png
magick icon@2x.png -fill "#ff4040" -stroke "#ff4040" -strokewidth 10 \
	-draw "line 28,116 116,28" icon-muted@2x.png

echo "icons regenerated"
```

- [ ] **Step 2: Generate the icons**

Run:
```bash
chmod +x scripts/generate-icons.sh
mkdir -p plugin/icons
scripts/generate-icons.sh
file plugin/icons/*.png
```
Expected: `icons regenerated`; `file` reports six PNGs (72x72 and 144x144 pairs).

- [ ] **Step 3: Commit**

```bash
git add scripts/generate-icons.sh plugin/icons
git commit -m "feat: add button icons and imagemagick generation script"
```

---

### Task 3: State machine (`src/state.rs`) — TDD

**Files:**
- Create: `src/state.rs`
- Modify: `src/main.rs` (add `mod state;`)

Pure logic, parity with the Python plugin: `muted: bool` (init false), `in_call: Option<bool>` (init None = unknown). Display: SETUP when unconfigured, OFF when not in an active call, MIC/MUTED otherwise.

- [ ] **Step 1: Write `src/state.rs` with the test module only** (types stubbed enough to compile is NOT possible here — write the full file skeleton with `todo!()` bodies and the complete tests)

```rust
//! Pure mute/in-call state machine and display mapping.

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

/// Icon variant for the button image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
	Normal,
	Muted,
	Off,
}

/// What the button should show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Display {
	pub state_index: u16,
	pub title: &'static str,
	pub icon: Icon,
}

impl MicState {
	pub fn in_active_call(&self) -> bool {
		self.in_call == Some(true)
	}

	/// Apply an MQTT payload. Returns true if the state changed.
	pub fn apply(&mut self, topic: StateTopic, payload: &str) -> bool {
		todo!()
	}
}

/// Map state to button display. `configured` gates the SETUP state.
pub fn display(mic: MicState, configured: bool) -> Display {
	todo!()
}

/// Whether a key press should publish toggle-mute.
pub fn can_toggle(mic: MicState, configured: bool) -> bool {
	todo!()
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

	// -- display mapping --

	#[test]
	fn display_unconfigured_is_setup() {
		let d = display(MicState::default(), false);
		assert_eq!(d, Display { state_index: 0, title: "SETUP", icon: Icon::Off });
	}

	#[test]
	fn display_unknown_call_state_is_off() {
		let d = display(MicState::default(), true);
		assert_eq!(d, Display { state_index: 0, title: "OFF", icon: Icon::Off });
	}

	#[test]
	fn display_not_in_call_is_off_even_if_muted() {
		let d = display(state(true, Some(false)), true);
		assert_eq!(d, Display { state_index: 0, title: "OFF", icon: Icon::Off });
	}

	#[test]
	fn display_in_call_unmuted_is_mic() {
		let d = display(state(false, Some(true)), true);
		assert_eq!(d, Display { state_index: 0, title: "MIC", icon: Icon::Normal });
	}

	#[test]
	fn display_in_call_muted_is_muted() {
		let d = display(state(true, Some(true)), true);
		assert_eq!(d, Display { state_index: 1, title: "MUTED", icon: Icon::Muted });
	}

	// -- toggle guard --

	#[test]
	fn can_toggle_only_when_configured_and_in_call() {
		assert!(can_toggle(state(false, Some(true)), true));
		assert!(can_toggle(state(true, Some(true)), true));
		assert!(!can_toggle(state(false, Some(true)), false));
		assert!(!can_toggle(state(false, Some(false)), true));
		assert!(!can_toggle(state(false, None), true));
	}
}
```

Also add to the top of `src/main.rs`:

```rust
mod state;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test state::`
Expected: panics at `todo!()` — every test fails (compilation succeeds).

- [ ] **Step 3: Implement the three `todo!()` bodies**

```rust
	/// Apply an MQTT payload. Returns true if the state changed.
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
```

```rust
pub fn display(mic: MicState, configured: bool) -> Display {
	if !configured {
		Display { state_index: 0, title: "SETUP", icon: Icon::Off }
	} else if !mic.in_active_call() {
		Display { state_index: 0, title: "OFF", icon: Icon::Off }
	} else if mic.muted {
		Display { state_index: 1, title: "MUTED", icon: Icon::Muted }
	} else {
		Display { state_index: 0, title: "MIC", icon: Icon::Normal }
	}
}

pub fn can_toggle(mic: MicState, configured: bool) -> bool {
	configured && mic.in_active_call()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test state::`
Expected: all 17 tests PASS. (`cargo build` will warn about unused items — they're consumed in Tasks 5–6.)

- [ ] **Step 5: Commit**

```bash
git add src/state.rs src/main.rs
git commit -m "feat: add mute/in-call state machine with display mapping (TDD)"
```

---

### Task 4: Settings resolution (`src/settings.rs`) — TDD

**Files:**
- Create: `src/settings.rs`
- Modify: `src/main.rs` (add `mod settings;`)

Layering: built-in defaults < config file (`config.toml`, with `password_file` indirection) < non-empty PI fields. `configured` = file exists ∨ any PI field set.

- [ ] **Step 1: Write `src/settings.rs` with full types, `todo!()` bodies, and complete tests**

```rust
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
		todo!()
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
	todo!()
}

/// Default path: `$XDG_CONFIG_HOME/opendeck-teams-for-linux/config.toml`
/// (falling back to `~/.config`).
pub fn config_file_path() -> PathBuf {
	let base = std::env::var_os("XDG_CONFIG_HOME")
		.map(PathBuf::from)
		.filter(|p| p.is_absolute())
		.unwrap_or_else(|| {
			PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
		});
	base.join("opendeck-teams-for-linux").join("config.toml")
}

/// Load and parse the config file. Returns None if it is missing or invalid
/// (invalid files are logged and treated as absent). Resolves `password_file`.
pub fn load_file_settings(path: &Path) -> Option<FileSettings> {
	todo!()
}

/// Layer: defaults < file < non-empty PI fields.
pub fn resolve(file: Option<&FileSettings>, pi: &PiSettings) -> Resolved {
	todo!()
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
		let mut pi = PiSettings::default();
		pi.username = "  ".into();
		assert!(!pi.any_set());
		pi.username = "geoff".into();
		assert!(pi.any_set());
	}

	#[test]
	fn join_topic_trims_slashes() {
		assert_eq!(join_topic("teams", "microphone"), "teams/microphone");
		assert_eq!(join_topic("/teams/", "/microphone/control/"), "teams/microphone/control");
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
		let mut pi = PiSettings::default();
		pi.username = "pi-user".into();
		let r = resolve(Some(&file), &pi);
		assert_eq!(r.broker_host, "10.0.0.5"); // empty PI field inherits
		assert_eq!(r.username, "pi-user"); // non-empty PI field wins
	}

	#[test]
	fn resolve_pi_only_marks_configured() {
		let mut pi = PiSettings::default();
		pi.broker_host = "127.0.0.1".into();
		let r = resolve(None, &pi);
		assert!(r.configured);
	}

	#[test]
	fn resolve_invalid_pi_port_falls_through() {
		let file = FileSettings { broker_port: Some(2000), ..Default::default() };
		let mut pi = PiSettings::default();
		pi.broker_port = "not-a-port".into();
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
			format!("password = \"inline\"\npassword_file = \"{}\"\n", pw_path.display()),
		)
		.unwrap();
		let fs = load_file_settings(&path).unwrap();
		assert_eq!(fs.password.as_deref(), Some("s3cret"));
	}

	#[test]
	fn load_unreadable_password_file_keeps_inline_password() {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("config.toml");
		std::fs::write(&path, "password = \"inline\"\npassword_file = \"/nonexistent/pw\"\n")
			.unwrap();
		let fs = load_file_settings(&path).unwrap();
		assert_eq!(fs.password.as_deref(), Some("inline"));
	}
}
```

Also add to `src/main.rs`:

```rust
mod settings;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test settings::`
Expected: panics at `todo!()` in `any_set`/`join_topic`/`load_file_settings`/`resolve`.

- [ ] **Step 3: Implement the `todo!()` bodies**

```rust
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
```

```rust
pub fn join_topic(prefix: &str, suffix: &str) -> String {
	format!("{}/{}", prefix.trim_matches('/'), suffix.trim_matches('/'))
}
```

```rust
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
			Ok(pw) => file.password = Some(pw.trim_end().to_string()),
			Err(err) => log::warn!("failed to read password_file: {err}"),
		}
	}
	Some(file)
}
```

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test settings::`
Expected: all 13 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs src/main.rs
git commit -m "feat: add layered settings resolution with config-file fallback (TDD)"
```

---

### Task 5: MQTT controller (`src/mqtt.rs`)

**Files:**
- Create: `src/mqtt.rs`
- Modify: `src/main.rs` (add `mod mqtt;`)

Glue around rumqttc: owns the connection task, applies settings changes (teardown + reconnect), exposes current display input over a watch channel, publishes toggle-mute. No unit tests beyond what Tasks 3–4 cover — verified by clippy here, then end-to-end by the Task 8 protocol test.

- [ ] **Step 1: Write `src/mqtt.rs`**

```rust
//! MQTT connection management: subscribe to state topics, publish commands.

use crate::settings::{self, PiSettings, Resolved};
use crate::state::{self, MicState, StateTopic};

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, watch};
use tokio::task::JoinHandle;

/// Everything the display pusher needs to render the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayInput {
	pub mic: MicState,
	pub configured: bool,
}

#[derive(Default)]
struct Inner {
	resolved: Option<Resolved>,
	client: Option<AsyncClient>,
	task: Option<JoinHandle<()>>,
}

pub struct MqttController {
	display_tx: watch::Sender<DisplayInput>,
	inner: Mutex<Inner>,
}

impl MqttController {
	pub fn new() -> (Arc<Self>, watch::Receiver<DisplayInput>) {
		let (display_tx, display_rx) = watch::channel(DisplayInput::default());
		let controller = Arc::new(Self { display_tx, inner: Mutex::new(Inner::default()) });
		(controller, display_rx)
	}

	/// Current display, computed from the latest state.
	pub fn current_display(&self) -> state::Display {
		let input = *self.display_tx.borrow();
		state::display(input.mic, input.configured)
	}

	/// Currently resolved settings (for PI placeholder reporting).
	pub async fn resolved(&self) -> Option<Resolved> {
		self.inner.lock().await.resolved.clone()
	}

	/// Re-resolve settings (file + PI) and reconnect if anything changed.
	pub async fn apply_settings(&self, pi: &PiSettings) {
		let file = settings::load_file_settings(&settings::config_file_path());
		let resolved = settings::resolve(file.as_ref(), pi);

		let mut inner = self.inner.lock().await;
		if inner.resolved.as_ref() == Some(&resolved) {
			return;
		}
		log::info!("settings changed; reconnecting mqtt");
		if let Some(task) = inner.task.take() {
			task.abort();
		}
		inner.client = None;

		// Reset state and propagate the configured flag; notifies the pusher.
		self.display_tx.send_modify(|d| {
			d.mic = MicState::default();
			d.configured = resolved.configured;
		});

		if resolved.configured {
			let (client, task) = spawn_mqtt(resolved.clone(), self.display_tx.clone());
			inner.client = Some(client);
			inner.task = Some(task);
		}
		inner.resolved = Some(resolved);
	}

	/// Handle a key press. Returns true if toggle-mute was published.
	pub async fn key_pressed(&self) -> bool {
		let inner = self.inner.lock().await;
		let input = *self.display_tx.borrow();
		if !state::can_toggle(input.mic, input.configured) {
			log::info!("ignoring key press: not in a call (or unconfigured)");
			return false;
		}
		let (Some(resolved), Some(client)) = (&inner.resolved, &inner.client) else {
			return false;
		};
		match client
			.publish(
				resolved.command_topic(),
				QoS::AtLeastOnce,
				false,
				r#"{"action":"toggle-mute"}"#,
			)
			.await
		{
			Ok(()) => {
				log::info!("published toggle-mute");
				true
			}
			Err(err) => {
				log::error!("mqtt publish failed: {err}");
				false
			}
		}
	}
}

/// Spawn the connection task: subscribe on connect, fold publishes into state,
/// retry forever with 5s backoff on errors.
fn spawn_mqtt(
	resolved: Resolved,
	display_tx: watch::Sender<DisplayInput>,
) -> (AsyncClient, JoinHandle<()>) {
	let mut options =
		MqttOptions::new("opendeck-teams-for-linux", resolved.broker_host.clone(), resolved.broker_port);
	options.set_keep_alive(Duration::from_secs(30));
	if !resolved.username.is_empty() {
		options.set_credentials(resolved.username.clone(), resolved.password.clone());
	}

	let (client, mut eventloop) = AsyncClient::new(options, 16);
	let subscriptions = [
		(resolved.microphone_topic(), StateTopic::Microphone),
		(resolved.microphone_control_topic(), StateTopic::MicrophoneControl),
		(resolved.in_call_topic(), StateTopic::InCall),
	];

	let subscribe_client = client.clone();
	let task = tokio::spawn(async move {
		loop {
			match eventloop.poll().await {
				Ok(Event::Incoming(Packet::ConnAck(_))) => {
					log::info!("mqtt connected");
					for (topic, _) in &subscriptions {
						if let Err(err) =
							subscribe_client.subscribe(topic.clone(), QoS::AtLeastOnce).await
						{
							log::error!("mqtt subscribe failed: {err}");
						}
					}
				}
				Ok(Event::Incoming(Packet::Publish(publish))) => {
					let payload =
						String::from_utf8_lossy(&publish.payload).trim().to_lowercase();
					log::debug!("mqtt rx {}={payload}", publish.topic);
					if let Some((_, topic)) =
						subscriptions.iter().find(|(t, _)| *t == publish.topic)
					{
						display_tx.send_if_modified(|d| d.mic.apply(*topic, &payload));
					}
				}
				Ok(_) => {}
				Err(err) => {
					log::warn!("mqtt error: {err}; retrying in 5s");
					tokio::time::sleep(Duration::from_secs(5)).await;
				}
			}
		}
	});

	(client, task)
}
```

Also add to `src/main.rs`:

```rust
mod mqtt;
```

- [ ] **Step 2: Verify it compiles cleanly**

Run: `cargo clippy --all-targets -- -D warnings 2>&1 | tail -20`
Expected: may emit dead-code warnings for not-yet-used controller methods (consumed in Task 6). If dead-code is the *only* failure class, proceed; anything else, fix it.

Run: `cargo test`
Expected: Tasks 3–4 tests still pass (30 tests).

- [ ] **Step 3: Commit**

```bash
git add src/mqtt.rs src/main.rs
git commit -m "feat: add mqtt controller with reconnecting state subscription"
```

---

### Task 6: Action + main wiring (`src/action.rs`, `src/main.rs`)

**Files:**
- Create: `src/action.rs`
- Modify: `src/main.rs` (full rewrite of the stub)

- [ ] **Step 1: Write `src/action.rs`**

```rust
//! OpenAction integration: the toggle-mute action and display pushing.

use crate::mqtt::{DisplayInput, MqttController};
use crate::state::{Display, Icon};

use base64::Engine as _;
use openaction::{Action, Instance, OpenActionResult, async_trait, visible_instances};
use std::sync::{Arc, LazyLock};
use tokio::sync::watch;

fn png_data_uri(bytes: &[u8]) -> String {
	format!(
		"data:image/png;base64,{}",
		base64::engine::general_purpose::STANDARD.encode(bytes)
	)
}

static ICON_NORMAL: LazyLock<String> =
	LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon@2x.png")));
static ICON_MUTED: LazyLock<String> =
	LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-muted@2x.png")));
static ICON_OFF: LazyLock<String> =
	LazyLock::new(|| png_data_uri(include_bytes!("../plugin/icons/icon-off@2x.png")));

fn icon_data_uri(icon: Icon) -> &'static str {
	match icon {
		Icon::Normal => &ICON_NORMAL,
		Icon::Muted => &ICON_MUTED,
		Icon::Off => &ICON_OFF,
	}
}

pub async fn push_display(instance: &Instance, display: Display) -> OpenActionResult<()> {
	instance.set_state(display.state_index).await?;
	instance.set_title(Some(display.title.to_string()), None).await?;
	instance
		.set_image(Some(icon_data_uri(display.icon).to_string()), None)
		.await
}

/// Watch for state changes and refresh every visible instance.
pub fn spawn_display_pusher(mut display_rx: watch::Receiver<DisplayInput>) {
	tokio::spawn(async move {
		while display_rx.changed().await.is_ok() {
			let input = *display_rx.borrow_and_update();
			let display = crate::state::display(input.mic, input.configured);
			for instance in visible_instances(ToggleMuteAction::UUID).await {
				if let Err(err) = push_display(&instance, display).await {
					log::warn!("display push failed: {err}");
				}
			}
		}
	});
}

pub struct ToggleMuteAction {
	pub controller: Arc<MqttController>,
}

#[async_trait]
impl Action for ToggleMuteAction {
	const UUID: &'static str = "com.geoffdavis.teamsforlinux.toggle-mute";
	type Settings = crate::settings::PiSettings;

	async fn will_appear(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.controller.apply_settings(settings).await;
		push_display(instance, self.controller.current_display()).await
	}

	async fn did_receive_settings(
		&self,
		instance: &Instance,
		settings: &Self::Settings,
	) -> OpenActionResult<()> {
		self.controller.apply_settings(settings).await;
		push_display(instance, self.controller.current_display()).await
	}

	async fn key_down(
		&self,
		instance: &Instance,
		_settings: &Self::Settings,
	) -> OpenActionResult<()> {
		if !self.controller.key_pressed().await {
			// Not in a call / unconfigured / publish failure: flash the warning triangle.
			instance.show_alert().await?;
		}
		Ok(())
	}

	async fn property_inspector_did_appear(
		&self,
		instance: &Instance,
		_settings: &Self::Settings,
	) -> OpenActionResult<()> {
		// Report effective values so the PI can show them as placeholders.
		if let Some(resolved) = self.controller.resolved().await {
			instance
				.send_to_property_inspector(serde_json::json!({
					"event": "effectiveSettings",
					"broker_host": resolved.broker_host,
					"broker_port": resolved.broker_port.to_string(),
					"username": resolved.username,
					"password_set": !resolved.password.is_empty(),
					"topic_prefix": resolved.topic_prefix,
					"microphone_topic": resolved.microphone_suffix,
					"microphone_control_topic": resolved.microphone_control_suffix,
					"in_call_topic": resolved.in_call_suffix,
					"command_topic": resolved.command_suffix,
				}))
				.await?;
		}
		Ok(())
	}
}
```

- [ ] **Step 2: Rewrite `src/main.rs`**

```rust
mod action;
mod mqtt;
mod settings;
mod state;

use openaction::{OpenActionResult, register_action, run};

#[tokio::main]
async fn main() -> OpenActionResult<()> {
	if let Err(err) = simplelog::TermLogger::init(
		simplelog::LevelFilter::Info,
		simplelog::Config::default(),
		simplelog::TerminalMode::Stderr,
		simplelog::ColorChoice::Never,
	) {
		eprintln!("logger initialization failed: {err}");
	}

	let (controller, display_rx) = mqtt::MqttController::new();
	action::spawn_display_pusher(display_rx);
	register_action(action::ToggleMuteAction { controller }).await;

	run(std::env::args().collect()).await
}
```

- [ ] **Step 3: Verify build, lints, and tests**

Run: `cargo clippy --all-targets -- -D warnings && cargo test && cargo fmt --check`
Expected: clean clippy (no remaining dead code — everything is wired now), 30 tests pass, formatting clean (run `cargo fmt` first if needed).

- [ ] **Step 4: Commit**

```bash
git add src/action.rs src/main.rs
git commit -m "feat: wire openaction action, display pusher, and entrypoint"
```

---

### Task 7: Manifest and property inspector

**Files:**
- Create: `plugin/manifest.json`
- Create: `plugin/pi/index.html`

- [ ] **Step 1: Write `plugin/manifest.json`**

```json
{
	"Name": "Teams for Linux",
	"Author": "Geoff Davis",
	"Version": "0.1.0",
	"Category": "Teams",
	"Icon": "icons/icon",
	"SDKVersion": 2,
	"OS": [{ "Platform": "linux" }],
	"CodePaths": {
		"x86_64-unknown-linux-gnu": "bin/plugin-x86_64",
		"aarch64-unknown-linux-gnu": "bin/plugin-aarch64"
	},
	"Actions": [
		{
			"UUID": "com.geoffdavis.teamsforlinux.toggle-mute",
			"Name": "Toggle Mute (Teams)",
			"Icon": "icons/icon",
			"Tooltip": "Toggle Microsoft Teams microphone mute via MQTT (teams-for-linux)",
			"Controllers": ["Keypad"],
			"DisableAutomaticStates": true,
			"PropertyInspectorPath": "pi/index.html",
			"States": [
				{ "Image": "icons/icon", "Title": "MIC", "FontSize": 18, "TitleColor": "#FFFFFF" },
				{ "Image": "icons/icon-muted", "Title": "MUTED", "FontSize": 18, "TitleColor": "#FF4040" }
			]
		}
	]
}
```

- [ ] **Step 2: Validate the manifest JSON**

Run: `jq . plugin/manifest.json > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 3: Write `plugin/pi/index.html`**

Input `id`s are the serde field names of `PiSettings` — that naming is a hard contract with `src/settings.rs`.

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Teams for Linux — Toggle Mute settings</title>
<style>
	:root { color-scheme: dark; }
	body {
		font-family: system-ui, sans-serif;
		background: #2d2d2d;
		color: #d8d8d8;
		font-size: 13px;
		margin: 8px;
	}
	.field { display: flex; align-items: center; margin-bottom: 6px; }
	.field label { flex: 0 0 110px; }
	.field input {
		flex: 1;
		background: #3d3d3d;
		color: #d8d8d8;
		border: 1px solid #555;
		border-radius: 4px;
		padding: 4px 6px;
	}
	details { margin-top: 10px; }
	summary { cursor: pointer; color: #aaa; }
	.hint { color: #888; font-size: 11px; margin-top: 10px; }
	code { color: #aaa; }
</style>
</head>
<body>
	<div class="field"><label for="broker_host">Broker host</label><input id="broker_host"></div>
	<div class="field"><label for="broker_port">Broker port</label><input id="broker_port" inputmode="numeric"></div>
	<div class="field"><label for="username">Username</label><input id="username"></div>
	<div class="field"><label for="password">Password</label><input id="password" type="password"></div>
	<div class="field"><label for="topic_prefix">Topic prefix</label><input id="topic_prefix"></div>
	<details>
		<summary>Advanced: topic overrides</summary>
		<div class="field"><label for="microphone_topic">Microphone</label><input id="microphone_topic"></div>
		<div class="field"><label for="microphone_control_topic">Mic control</label><input id="microphone_control_topic"></div>
		<div class="field"><label for="in_call_topic">In call</label><input id="in_call_topic"></div>
		<div class="field"><label for="command_topic">Command</label><input id="command_topic"></div>
	</details>
	<p class="hint">
		Empty fields inherit from <code>~/.config/opendeck-teams-for-linux/config.toml</code>
		or built-in defaults; placeholders show the effective values.
	</p>
<script>
	const FIELDS = [
		"broker_host", "broker_port", "username", "password", "topic_prefix",
		"microphone_topic", "microphone_control_topic", "in_call_topic", "command_topic",
	];
	let ws = null;
	let ctx = null;
	let saveTimer = null;

	// Called by OpenDeck's injected shim (Stream Deck SDK compatible entry point).
	function connectElgatoStreamDeckSocket(inPort, inUUID, inRegisterEvent, inInfo, inActionInfo) {
		ctx = inUUID;
		const actionInfo = JSON.parse(inActionInfo);
		populate(actionInfo.payload?.settings ?? {});
		ws = new WebSocket("ws://127.0.0.1:" + inPort);
		ws.onopen = () => ws.send(JSON.stringify({ event: inRegisterEvent, uuid: inUUID }));
		ws.onmessage = (e) => {
			const msg = JSON.parse(e.data);
			if (msg.event === "didReceiveSettings") {
				populate(msg.payload?.settings ?? {});
			} else if (msg.event === "sendToPropertyInspector" && msg.payload?.event === "effectiveSettings") {
				placeholders(msg.payload);
			}
		};
	}

	function populate(settings) {
		for (const f of FIELDS) document.getElementById(f).value = settings[f] ?? "";
	}

	function placeholders(eff) {
		for (const f of FIELDS) {
			if (f === "password") continue;
			document.getElementById(f).placeholder = eff[f] ?? "";
		}
		document.getElementById("password").placeholder = eff.password_set ? "(set)" : "";
	}

	function save() {
		if (!ws || ws.readyState !== WebSocket.OPEN) return;
		const settings = {};
		for (const f of FIELDS) settings[f] = document.getElementById(f).value.trim();
		ws.send(JSON.stringify({ event: "setSettings", context: ctx, payload: settings }));
	}

	for (const f of FIELDS) {
		document.getElementById(f).addEventListener("input", () => {
			clearTimeout(saveTimer);
			saveTimer = setTimeout(save, 300);
		});
	}
</script>
</body>
</html>
```

- [ ] **Step 4: Commit**

```bash
git add plugin/manifest.json plugin/pi/index.html
git commit -m "feat: add plugin manifest and property inspector"
```

---

### Task 8: Protocol-level integration test (`tests/protocol.rs`)

**Files:**
- Create: `tests/protocol.rs`

Runs the **real plugin binary** against a mock OpenDeck WebSocket server and a throwaway mosquitto broker (from the dev shell). No OpenDeck install or hardware needed — this is the executable proof the glue works, here and in CI. Event JSON shapes below are verified against the SDK source (`AppearEvent`/`KeyEvent` with camelCase `GenericInstancePayload`; `-info` must parse as `{"devices":[]}`; register message echoes the `-registerEvent` value with the plugin UUID).

- [ ] **Step 1: Write `tests/protocol.rs`**

```rust
//! End-to-end protocol test: the real plugin binary against a mock OpenDeck
//! WebSocket server and a throwaway mosquitto broker. No OpenDeck
//! installation or Stream Deck hardware required.

use futures_util::{SinkExt, StreamExt};
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

const ACTION_UUID: &str = "com.geoffdavis.teamsforlinux.toggle-mute";

struct ChildGuard(std::process::Child);
impl Drop for ChildGuard {
	fn drop(&mut self) {
		let _ = self.0.kill();
		let _ = self.0.wait();
	}
}

fn free_port() -> u16 {
	std::net::TcpListener::bind("127.0.0.1:0")
		.unwrap()
		.local_addr()
		.unwrap()
		.port()
}

type WsStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

/// Read frames until one matches `predicate`, panicking after 10s.
async fn expect_message<F: Fn(&Value) -> bool>(
	ws: &mut WsStream,
	what: &str,
	predicate: F,
) -> Value {
	timeout(Duration::from_secs(10), async {
		loop {
			let frame = ws.next().await.expect("ws closed").expect("ws error");
			if let Message::Text(text) = frame {
				let value: Value =
					serde_json::from_str(&text).expect("invalid json from plugin");
				if predicate(&value) {
					return value;
				}
			}
		}
	})
	.await
	.unwrap_or_else(|_| panic!("timed out waiting for {what}"))
}

async fn expect_title(ws: &mut WsStream, title: &str) {
	expect_message(ws, &format!("setTitle {title}"), |v| {
		v["event"] == "setTitle" && v["payload"]["title"] == title
	})
	.await;
}

fn instance_event(event: &str) -> String {
	json!({
		"event": event,
		"action": ACTION_UUID,
		"context": "ctx1",
		"device": "dev1",
		"payload": {
			"settings": {},
			"controller": "Keypad",
			"state": 0,
			"isInMultiAction": false,
		},
	})
	.to_string()
}

#[tokio::test]
async fn plugin_speaks_protocol_end_to_end() {
	// 1. Throwaway mosquitto broker on a free port.
	let mqtt_port = free_port();
	let dir = tempfile::tempdir().unwrap();
	let conf = dir.path().join("mosquitto.conf");
	std::fs::write(
		&conf,
		format!("listener {mqtt_port} 127.0.0.1\nallow_anonymous true\npersistence false\n"),
	)
	.unwrap();
	let _broker = ChildGuard(
		std::process::Command::new("mosquitto")
			.arg("-c")
			.arg(&conf)
			.spawn()
			.expect("mosquitto not found - enter the dev shell"),
	);
	timeout(Duration::from_secs(10), async {
		while tokio::net::TcpStream::connect(("127.0.0.1", mqtt_port)).await.is_err() {
			sleep(Duration::from_millis(100)).await;
		}
	})
	.await
	.expect("mosquitto did not come up");

	// 2. Config file marks the plugin configured (XDG redirected to the tempdir).
	let xdg = dir.path().join("xdg");
	std::fs::create_dir_all(xdg.join("opendeck-teams-for-linux")).unwrap();
	std::fs::write(
		xdg.join("opendeck-teams-for-linux/config.toml"),
		format!("broker_host = \"127.0.0.1\"\nbroker_port = {mqtt_port}\n"),
	)
	.unwrap();

	// 3. Mock OpenDeck WebSocket server.
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let ws_port = listener.local_addr().unwrap().port();

	// 4. The real plugin binary.
	let _plugin = ChildGuard(
		std::process::Command::new(env!("CARGO_BIN_EXE_opendeck-teams-for-linux"))
			.args([
				"-port",
				&ws_port.to_string(),
				"-pluginUUID",
				"test-plugin-uuid",
				"-registerEvent",
				"registerPlugin",
				"-info",
				r#"{"devices":[]}"#,
			])
			.env("XDG_CONFIG_HOME", &xdg)
			.spawn()
			.unwrap(),
	);

	let (stream, _) = timeout(Duration::from_secs(10), listener.accept())
		.await
		.expect("plugin did not connect")
		.unwrap();
	let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();

	// 5. Registration handshake.
	let register =
		expect_message(&mut ws, "register", |v| v["event"] == "registerPlugin").await;
	assert_eq!(register["uuid"], "test-plugin-uuid");

	// 6. Instance appears; file => configured, call state unknown => OFF.
	ws.send(Message::text(instance_event("willAppear"))).await.unwrap();
	expect_title(&mut ws, "OFF").await;

	// 7. Drive Teams state over MQTT (retained => plugin subscribe order is irrelevant).
	let mut options = MqttOptions::new("test-driver", "127.0.0.1", mqtt_port);
	options.set_keep_alive(Duration::from_secs(10));
	let (client, mut eventloop) = AsyncClient::new(options, 16);
	let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
	tokio::spawn(async move {
		loop {
			match eventloop.poll().await {
				Ok(Event::Incoming(Packet::Publish(p))) => {
					let _ = cmd_tx.send(String::from_utf8_lossy(&p.payload).to_string());
				}
				Ok(_) => {}
				Err(_) => break,
			}
		}
	});
	client.subscribe("teams/command", QoS::AtLeastOnce).await.unwrap();

	client.publish("teams/in-call", QoS::AtLeastOnce, true, "true").await.unwrap();
	expect_title(&mut ws, "MIC").await;

	client
		.publish("teams/microphone/control", QoS::AtLeastOnce, true, "muted")
		.await
		.unwrap();
	expect_title(&mut ws, "MUTED").await;

	client
		.publish("teams/microphone/control", QoS::AtLeastOnce, true, "unmuted")
		.await
		.unwrap();
	expect_title(&mut ws, "MIC").await;

	// 8. Key press while in a call publishes toggle-mute.
	ws.send(Message::text(instance_event("keyDown"))).await.unwrap();
	let command = timeout(Duration::from_secs(10), cmd_rx.recv())
		.await
		.expect("timed out waiting for toggle-mute")
		.unwrap();
	assert_eq!(
		serde_json::from_str::<Value>(&command).unwrap()["action"],
		"toggle-mute"
	);

	// 9. Call ends => OFF; key presses now no-op (plugin sends showAlert, not a publish).
	client.publish("teams/in-call", QoS::AtLeastOnce, true, "false").await.unwrap();
	expect_title(&mut ws, "OFF").await;
	ws.send(Message::text(instance_event("keyDown"))).await.unwrap();
	expect_message(&mut ws, "showAlert", |v| v["event"] == "showAlert").await;
}
```

- [ ] **Step 2: Run the integration test**

Run: `cargo test --test protocol -- --nocapture`
Expected: PASS in a few seconds (it spawns mosquitto from the dev shell and the freshly built plugin binary).

- [ ] **Step 3: Run the full test suite**

Run: `cargo test`
Expected: 30 unit tests + 1 integration test PASS.

- [ ] **Step 4: Commit**

```bash
git add tests/protocol.rs
git commit -m "test: add end-to-end protocol test with mock OpenDeck and live broker"
```

---

### Task 9: Assembly script

**Files:**
- Create: `scripts/assemble-plugin.sh`

- [ ] **Step 1: Write `scripts/assemble-plugin.sh`**

```bash
#!/usr/bin/env bash
# Assemble dist/com.geoffdavis.teamsforlinux.sdPlugin from plugin/ assets and
# built binaries. Prefers musl release binaries (CI); falls back to the native
# release binary for local smoke testing.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/dist/com.geoffdavis.teamsforlinux.sdPlugin"

rm -rf "$dist"
mkdir -p "$dist/bin"
cp -r "$root/plugin/." "$dist/"

copied=0
for arch in x86_64 aarch64; do
	bin="$root/target/${arch}-unknown-linux-musl/release/opendeck-teams-for-linux"
	if [[ -f "$bin" ]]; then
		cp "$bin" "$dist/bin/plugin-${arch}"
		copied=1
	fi
done

if [[ "$copied" -eq 0 ]]; then
	bin="$root/target/release/opendeck-teams-for-linux"
	if [[ ! -f "$bin" ]]; then
		echo "no built binaries found; run cargo build --release first" >&2
		exit 1
	fi
	cp "$bin" "$dist/bin/plugin-$(uname -m)"
fi

echo "assembled $dist"
```

- [ ] **Step 2: Build and assemble locally**

Run:
```bash
chmod +x scripts/assemble-plugin.sh
cargo build --release
scripts/assemble-plugin.sh
ls dist/com.geoffdavis.teamsforlinux.sdPlugin/{manifest.json,bin,icons,pi}
```
Expected: `assembled …`; listing shows manifest, `bin/plugin-x86_64`, six icons, `pi/index.html`.

- [ ] **Step 3: Commit**

```bash
git add scripts/assemble-plugin.sh
git commit -m "feat: add sdPlugin assembly script"
```

---

### Task 10: Flake package output + home-manager module

**Files:**
- Modify: `flake.nix`
- Create: `nix/hm-module.nix`

- [ ] **Step 1: Add `packages` and `homeManagerModules` outputs to `flake.nix`**

Full new file content:

```nix
{
  description = "OpenDeck plugin: Microsoft Teams (teams-for-linux) mute button with MQTT-driven state";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["x86_64-linux" "aarch64-linux"];
    eachSystem = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    pluginId = "com.geoffdavis.teamsforlinux.sdPlugin";
  in {
    packages = eachSystem (pkgs: {
      default = pkgs.rustPlatform.buildRustPackage {
        pname = "opendeck-teams-for-linux";
        version = "0.1.0";
        src = self;
        cargoLock.lockFile = ./Cargo.lock;

        # Assemble the .sdPlugin directory next to the raw binary. The binary
        # is named by the host gnu triple key OpenDeck looks up in CodePaths.
        postInstall = ''
          plugin="$out/share/opendeck-teams-for-linux/${pluginId}"
          mkdir -p "$plugin/bin"
          cp -r plugin/. "$plugin/"
          cp "$out/bin/opendeck-teams-for-linux" \
            "$plugin/bin/plugin-${pkgs.stdenv.hostPlatform.parsed.cpu.name}"
        '';

        meta = {
          description = "OpenDeck plugin for teams-for-linux mute control via MQTT";
          homepage = "https://github.com/geoffdavis/opendeck-teams-for-linux";
          license = pkgs.lib.licenses.mit;
          platforms = systems;
        };
      };
    });

    homeManagerModules.default = import ./nix/hm-module.nix self;

    devShells = eachSystem (pkgs: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          rustc
          clippy
          rustfmt
          rust-analyzer
          imagemagick
          zip
          jq
          mosquitto
        ];
      };
    });

    formatter = eachSystem (pkgs: pkgs.alejandra);
  };
}
```

- [ ] **Step 2: Write `nix/hm-module.nix`**

```nix
# Home-manager module: installs the plugin into OpenDeck's plugin directory
# and optionally writes the declarative config file the plugin layers under
# property-inspector settings.
self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.opendeck-teams-for-linux;
  tomlFormat = pkgs.formats.toml {};
  pluginId = "com.geoffdavis.teamsforlinux.sdPlugin";
in {
  options.programs.opendeck-teams-for-linux = {
    enable = lib.mkEnableOption "OpenDeck plugin for teams-for-linux mute control";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      defaultText = lib.literalExpression "opendeck-teams-for-linux.packages.<system>.default";
      description = "Plugin package to install.";
    };

    settings = lib.mkOption {
      type = tomlFormat.type;
      default = {};
      example = lib.literalExpression ''
        {
          username = "teams-for-linux";
          password_file = "/home/me/.config/opendeck-teams-for-linux/password";
          topic_prefix = "teams";
        }
      '';
      description = ''
        Contents of {file}`~/.config/opendeck-teams-for-linux/config.toml`.
        Resolution order: built-in defaults < this file < non-empty property
        inspector fields. Use {var}`password_file` (not {var}`password`) to
        keep secrets out of the Nix store.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    xdg.configFile."opendeck/plugins/${pluginId}".source =
      "${cfg.package}/share/opendeck-teams-for-linux/${pluginId}";

    xdg.configFile."opendeck-teams-for-linux/config.toml" = lib.mkIf (cfg.settings != {}) {
      source = tomlFormat.generate "opendeck-teams-for-linux-config.toml" cfg.settings;
    };
  };
}
```

- [ ] **Step 3: Build and inspect the package**

Run:
```bash
nix build .#default
ls result/share/opendeck-teams-for-linux/com.geoffdavis.teamsforlinux.sdPlugin/{manifest.json,bin,icons,pi}
nix flake check
```
Expected: build succeeds; sdPlugin dir contains manifest, `bin/plugin-x86_64`, icons, PI; flake check passes.

- [ ] **Step 4: Commit**

```bash
git add flake.nix nix/hm-module.nix
git commit -m "feat: add flake package output and home-manager module"
```

---

### Task 11: CI and release workflows

**Files:**
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Write `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      # mosquitto is required by tests/protocol.rs (spawned on a free port)
      - run: sudo apt-get update && sudo apt-get install -y mosquitto
      - run: cargo fmt --all --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl,aarch64-unknown-linux-musl
      - run: pip3 install cargo-zigbuild
      - run: cargo zigbuild --release --target x86_64-unknown-linux-musl
      - run: cargo zigbuild --release --target aarch64-unknown-linux-musl
      - run: scripts/assemble-plugin.sh
      - uses: actions/upload-artifact@v4
        with:
          name: com.geoffdavis.teamsforlinux.sdPlugin
          path: dist/com.geoffdavis.teamsforlinux.sdPlugin
```

- [ ] **Step 2: Write `.github/workflows/release.yml`**

```yaml
name: Release

on:
  push:
    tags: ["v*"]

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: x86_64-unknown-linux-musl,aarch64-unknown-linux-musl
      - run: pip3 install cargo-zigbuild
      - run: cargo zigbuild --release --target x86_64-unknown-linux-musl
      - run: cargo zigbuild --release --target aarch64-unknown-linux-musl
      - run: scripts/assemble-plugin.sh
      - run: cd dist && zip -r com.geoffdavis.teamsforlinux.sdPlugin.zip com.geoffdavis.teamsforlinux.sdPlugin
      - uses: softprops/action-gh-release@v2
        with:
          files: dist/com.geoffdavis.teamsforlinux.sdPlugin.zip
          generate_release_notes: true
```

- [ ] **Step 3: Validate workflow syntax locally**

Run: `nix run nixpkgs#action-validator -- .github/workflows/ci.yml .github/workflows/release.yml || true`
Expected: no schema errors (if action-validator is unavailable, `yq . file` via `nix run nixpkgs#yq-go` as a YAML-parse sanity check is sufficient).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows
git commit -m "ci: add lint/test/build workflow and tag-driven release workflow"
```

---

### Task 12: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Write `README.md`**

```markdown
# opendeck-teams-for-linux

An [OpenDeck](https://github.com/nekename/OpenDeck) (Stream Deck) plugin for
[teams-for-linux](https://github.com/IsmaelMartinez/teams-for-linux): a mute
button that shows your **live** Microsoft Teams microphone state and toggles
mute, driven by teams-for-linux's MQTT integration.

| Button | Meaning |
|--------|---------|
| `MIC` (purple) | In a call, microphone live |
| `MUTED` (red slash) | In a call, microphone muted |
| `OFF` (grey) | Not in a call — presses are ignored |
| `SETUP` (grey) | Not configured yet |

## Requirements

- Linux with OpenDeck ≥ 2.x and a Stream Deck (or compatible) device
- teams-for-linux with MQTT enabled (see below)
- An MQTT broker — typically [mosquitto](https://mosquitto.org/) listening on
  localhost

## Install

Download `com.geoffdavis.teamsforlinux.sdPlugin.zip` from
[Releases](https://github.com/geoffdavis/opendeck-teams-for-linux/releases) and
install it from OpenDeck's plugins view, or unzip it into
`~/.config/opendeck/plugins/`.

## Configure teams-for-linux

In teams-for-linux's `config.json`
([docs](https://ismaelmartinez.github.io/teams-for-linux/mqtt-integration)):

​```json
{
  "mqtt": {
    "enabled": true,
    "brokerUrl": "mqtt://127.0.0.1:1883",
    "username": "teams-for-linux",
    "password": "<broker password>",
    "topicPrefix": "teams",
    "commandTopic": "command"
  }
}
​```

The default topics (`microphone`, `microphone/control`, `in-call`, `command`
under the `teams` prefix) match this plugin's defaults — if you customize
`mqtt.mediaTopics`, mirror the values in the plugin settings.

## Configure the plugin

Open the button's property inspector in OpenDeck and fill in the broker
credentials. Empty fields inherit from the optional config file or built-in
defaults (host `127.0.0.1`, port `1883`, prefix `teams`).

### Declarative config file (optional)

`~/.config/opendeck-teams-for-linux/config.toml` — every key optional:

​```toml
broker_host = "127.0.0.1"
broker_port = 1883
username = "teams-for-linux"
# Prefer password_file over password to keep secrets out of world-readable config:
password_file = "/home/me/.config/opendeck-teams-for-linux/password"
topic_prefix = "teams"
# Advanced topic suffix overrides:
# microphone_topic = "microphone"
# microphone_control_topic = "microphone/control"
# in_call_topic = "in-call"
# command_topic = "command"
​```

Resolution order: built-in defaults < config file < non-empty property
inspector fields.

> **Security note:** values entered in the property inspector (including the
> password) are stored as plain JSON in OpenDeck's settings store, equivalent
> in trust to a `0600` config file. The intended model is a localhost-only
> broker; TLS/remote brokers are planned for a later release.

## Nix / home-manager

​```nix
{
  inputs.opendeck-teams-for-linux.url = "github:geoffdavis/opendeck-teams-for-linux";

  # in home-manager configuration:
  imports = [inputs.opendeck-teams-for-linux.homeManagerModules.default];

  programs.opendeck-teams-for-linux = {
    enable = true;
    settings = {
      username = "teams-for-linux";
      password_file = "/home/me/.config/opendeck-teams-for-linux/password";
    };
  };
}
​```

The module installs the plugin into `~/.config/opendeck/plugins/` and writes
the config file; populate `password_file` outside the Nix store (e.g. an
activation script reading your secrets manager).

## Development

​```bash
direnv allow          # rust toolchain, imagemagick, mosquitto via nix-direnv
cargo test            # state machine + settings resolution
cargo build --release
scripts/assemble-plugin.sh   # -> dist/com.geoffdavis.teamsforlinux.sdPlugin
scripts/generate-icons.sh    # regenerate committed icons
​```

How it works: a background task subscribes to the state topics and folds
payloads into a `{muted, in_call}` state machine; a watch channel fans changes
out to every visible button instance. Key presses publish
`{"action":"toggle-mute"}` to the command topic, guarded by in-call state.
teams-for-linux's `microphone/control` topic (muted/unmuted/off) is the
authoritative mute signal.

## License

[MIT](LICENSE)
​```

(Note for the executor: the ​``` fences above are escaped for plan nesting — write real triple-backtick fences in the actual README.)

- [ ] **Step 2: Sanity-render check**

Run: `nix run nixpkgs#cmark-gfm -- -e table README.md > /dev/null && echo OK`
Expected: `OK`

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: add README with install, config, nix, and development guides"
```

---

### Task 13: Publish — GitHub repo, push, tag v0.1.0

**CONFIRM WITH GEOFF before this task — it is outward-facing (public repo + release).**

- [ ] **Step 1: Create the GitHub repository and push**

Run:
```bash
cd ~/src/opendeck-teams-for-linux
gh repo create geoffdavis/opendeck-teams-for-linux --public \
	--description "OpenDeck (Stream Deck) mute button for teams-for-linux, driven by MQTT" \
	--source . --push
```
Expected: repo created, main pushed.

- [ ] **Step 2: Verify CI passes on main**

Run: `gh run watch --repo geoffdavis/opendeck-teams-for-linux || gh run list --repo geoffdavis/opendeck-teams-for-linux --limit 3`
Expected: CI workflow green (check + build jobs).

- [ ] **Step 3: Tag and release v0.1.0**

Run:
```bash
git tag v0.1.0
git push origin v0.1.0
gh run watch --repo geoffdavis/opendeck-teams-for-linux
gh release view v0.1.0 --repo geoffdavis/opendeck-teams-for-linux
```
Expected: release workflow green; release v0.1.0 exists with
`com.geoffdavis.teamsforlinux.sdPlugin.zip` attached.

- [ ] **Step 4: Verify the released artifact installs**

Run:
```bash
cd /tmp && rm -rf odtfl-test && mkdir odtfl-test && cd odtfl-test
gh release download v0.1.0 --repo geoffdavis/opendeck-teams-for-linux
unzip -l com.geoffdavis.teamsforlinux.sdPlugin.zip
```
Expected: zip contains `com.geoffdavis.teamsforlinux.sdPlugin/` with manifest,
both `bin/plugin-*` binaries, icons, and `pi/index.html`.

---

### Follow-up A: Hardware smoke test (work machine — the only box with OpenDeck + deck + Teams stack)

After v0.1.0 is released, on the machine managed by the consumer repo:

1. Download and unzip the release into `~/.config/opendeck/plugins/` (next to
   the existing `com.gdavis.teamsmute.sdPlugin` — different UUID, no conflict):
   ```bash
   cd ~/.config/opendeck/plugins
   gh release download v0.1.0 --repo geoffdavis/opendeck-teams-for-linux
   unzip com.geoffdavis.teamsforlinux.sdPlugin.zip && rm com.geoffdavis.teamsforlinux.sdPlugin.zip
   ```
2. Write the config file from the existing credentials:
   ```bash
   umask 077
   mkdir -p ~/.config/opendeck-teams-for-linux
   grep '^password=' ~/.config/teams-mute/mqtt.env | cut -d= -f2- \
   	> ~/.config/opendeck-teams-for-linux/password
   cat > ~/.config/opendeck-teams-for-linux/config.toml <<EOF
   username = "teams-for-linux"
   password_file = "$HOME/.config/opendeck-teams-for-linux/password"
   EOF
   ```
3. Restart OpenDeck, add "Toggle Mute (Teams)" to a free key — expect `OFF`.
4. Simulate state and watch the deck key:
   ```bash
   PW="$(cat ~/.config/opendeck-teams-for-linux/password)"
   mosquitto_pub -u teams-for-linux -P "$PW" -t teams/in-call -m true -r          # -> MIC
   mosquitto_pub -u teams-for-linux -P "$PW" -t teams/microphone/control -m muted -r   # -> MUTED
   mosquitto_pub -u teams-for-linux -P "$PW" -t teams/microphone/control -m unmuted -r # -> MIC
   ```
   Press the key with `mosquitto_sub … -t teams/command` running — expect
   `{"action":"toggle-mute"}`. Set `teams/in-call` to `false` — expect `OFF`
   and the alert triangle on press.
5. Open the property inspector: placeholders show effective values (host
   `127.0.0.1`, username `teams-for-linux`, password `(set)`).
6. Clear simulated retained messages, then verify against a real Teams call:
   ```bash
   mosquitto_pub -u teams-for-linux -P "$PW" -t teams/in-call -r -n
   mosquitto_pub -u teams-for-linux -P "$PW" -t teams/microphone/control -r -n
   ```

### Follow-up B: consumer-repo migration (separate plan, different repo)

Migrate the consumer repo to consume the published flake (HM module +
`settings`), delete `modules/teams-mute-plugin.py` and the plugin wiring in
`modules/teams-for-linux.nix`, and remove the old `com.gdavis.teamsmute`
button — see spec § "Migration plan (consumer repo)". Do this only after
Follow-up A passes.
