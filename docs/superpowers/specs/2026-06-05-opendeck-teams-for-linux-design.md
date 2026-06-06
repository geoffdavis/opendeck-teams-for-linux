# opendeck-teams-for-linux — Design

**Date:** 2026-06-05
**Status:** Approved
**Repo:** github.com/geoffdavis/opendeck-teams-for-linux (to be created)

## Context

An OpenDeck (Stream Deck) mute button for [teams-for-linux](https://github.com/IsmaelMartinez/teams-for-linux)
currently lives as a private Python script + Nix module inside a private consumer repo
(`modules/teams-mute-plugin.py`, `modules/teams-for-linux.nix`). It rides on
teams-for-linux's MQTT integration — specifically the `microphone/control` state
topic and command topic introduced in teams-for-linux PR #2608 — to show live
MIC / MUTED / OFF state and toggle mute on key press.

This design extracts the plugin into a dedicated public repository so any
OpenDeck user can install it, while keeping a first-class Nix consumption path
for declarative setups.

## Decisions (settled during brainstorming)

| Decision | Choice |
|----------|--------|
| Audience | Any OpenDeck user (zip release) **and** Nix users (flake + home-manager module) |
| Implementation | Rust rewrite using the [`openaction`](https://crates.io/crates/openaction) SDK crate (v2.6.x, maintained by the OpenAction org) + `rumqttc` |
| Configuration | Property inspector, layered over an optional config file for declarative setups |
| Repo name | `opendeck-teams-for-linux` |
| Plugin UUID | `com.geoffdavis.teamsforlinux`; action `com.geoffdavis.teamsforlinux.toggle-mute` (room for future sibling actions) |
| License | MIT |
| Store listing | Deferred until the teams-for-linux PR (#2608) is merged |

## Goals

- Ready-to-install `.sdPlugin` zip on GitHub Releases; no runtime dependencies
  (static musl binaries, x86_64 + aarch64 in one zip).
- Property-inspector configuration usable by non-technical users.
- Declarative pre-configuration via home-manager so the button works without
  ever opening the PI.
- Behavior parity with the existing Python plugin (state model, guards,
  reconnect semantics).

## Non-goals

- TLS/remote MQTT brokers in v1 (the model is a localhost broker; planned as
  a later release — see Future ideas).
- Managing the broker, teams-for-linux `config.json`, or secrets — host
  concerns that stay in the consumer's config (e.g. the consumer repos).
- Additional actions (camera, presence display) — the UUID scheme leaves room,
  but v1 ships only toggle-mute.
- OpenDeck store listing (deferred; see Decisions).

## Repository layout

```
opendeck-teams-for-linux/
├── flake.nix / flake.lock      # package, home-manager module, devShell
├── .envrc                      # use flake (nix-direnv convention)
├── Cargo.toml / Cargo.lock
├── src/
│   ├── main.rs                 # logging init, register action, openaction::run
│   ├── action.rs               # ToggleMuteAction: key_down, will_appear/disappear, settings events
│   ├── mqtt.rs                 # rumqttc task: subscribe, publish, retry loop
│   ├── settings.rs             # Settings struct + defaults < file < PI resolution
│   └── state.rs                # pure state machine: payloads → state → display
├── tests/protocol.rs           # e2e: mock OpenDeck WS + throwaway mosquitto + real binary
├── plugin/
│   ├── manifest.json
│   ├── pi/index.html           # property inspector (vanilla JS, sdpi-style layout)
│   └── icons/                  # committed PNGs: icon, icon-muted, icon-off (+@2x)
├── scripts/generate-icons.sh   # imagemagick recipe (port of the nix drawIcon derivations)
├── .github/workflows/ci.yml
├── .github/workflows/release.yml
├── docs/superpowers/specs/     # this document
├── LICENSE                     # MIT
└── README.md
```

## Runtime architecture

- `main.rs` registers `ToggleMuteAction` with the `openaction` SDK and calls
  `run()`, which owns the plugin↔OpenDeck WebSocket.
- One background MQTT task (tokio) owns the `rumqttc` connection. It is
  (re)started whenever resolved settings change and retries forever with 5 s
  backoff on failure (parity with the Python `_mqtt_loop`).
- The MQTT task publishes state changes over a `tokio::sync::watch` channel.
  The action layer watches it and pushes `set_state` / `set_title` /
  `set_image` to every visible instance (tracked via
  `will_appear`/`will_disappear`).
- `key_down` publishes `{"action":"toggle-mute"}` (QoS 1) to the command topic,
  guarded by in-call state.

### State machine (parity with the Python plugin)

Internal state: `muted: bool` (init `false`), `in_call: Option<bool>` (init
`None` = unknown).

Subscribed topics (QoS 1) and transitions:

| Topic | Payload | Effect |
|-------|---------|--------|
| `{prefix}/{microphone_control}` | `muted` | `muted = true` |
| | `unmuted` | `muted = false` |
| | `off` | `in_call = false`, `muted = false` |
| `{prefix}/{microphone}` | `true`, `muted` | `muted = true` |
| | `false`, `speaking`, `silent` | `muted = false` |
| | `off` | `in_call = false`, `muted = false` |
| | anything else | ignored |
| `{prefix}/{in_call}` | `true` | `in_call = true` |
| | `false` | `in_call = false`; if muted, reset `muted = false` |

Display mapping (recomputed on every transition and on `will_appear`):

| Condition | State index | Title | Icon |
|-----------|-------------|-------|------|
| Unconfigured (no usable settings) | 0 | `SETUP` | icon-off |
| `in_call != Some(true)` | 0 | `OFF` | icon-off |
| in call, unmuted | 0 | `MIC` | icon |
| in call, muted | 1 | `MUTED` | icon-muted |

Key press: no-op unless configured **and** `in_call == Some(true)`.

## Settings

```rust
struct Settings {
    broker_host: String,              // default "127.0.0.1"
    broker_port: u16,                 // default 1883
    username: String,                 // default ""
    password: String,                 // default ""
    topic_prefix: String,             // default "teams"
    microphone_topic: String,         // default "microphone"
    microphone_control_topic: String, // default "microphone/control"
    in_call_topic: String,            // default "in-call"
    command_topic: String,            // default "command"
}
```

Topic defaults mirror teams-for-linux's `mqtt.mediaTopics` defaults.

### Resolution: defaults < config file < non-empty PI fields

- Optional config file: `~/.config/opendeck-teams-for-linux/config.toml`
  (override path honors `$XDG_CONFIG_HOME`). Keys as in the struct above, all
  optional, plus `password_file` (path whose contents become the password;
  wins over `password` if both set). This is the declarative-setup hook.
- PI fields left empty inherit from file/defaults; the PI shows the inherited
  effective values as input placeholders.
- An instance is "configured" iff the config file exists **or** at least one
  PI field is non-empty. Fresh install (no file, untouched PI) → `SETUP`
  state, no MQTT connection attempted.
- On any settings change (PI event or differing file content at instance
  appear), the MQTT task reconnects with the new parameters.

### Security note

PI-entered passwords live in OpenDeck's settings store as plain JSON — the
same trust level as the current `mqtt.env` (0600 file). The README documents
this and recommends the localhost-broker model. `password_file` keeps secrets
out of the nix store for HM users.

## Property inspector

`plugin/pi/index.html`, vanilla JS, sdpi-compatible styling. Fields: broker
host, port, username, password (masked), topic prefix; collapsed "Advanced"
section with the four topic-suffix overrides. Registers over the PI WebSocket,
loads current settings via `didReceiveSettings`, persists with `setSettings`
(debounced on input).

## Manifest

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
  "Actions": [{
    "UUID": "com.geoffdavis.teamsforlinux.toggle-mute",
    "Name": "Toggle Mute (Teams)",
    "Icon": "icons/icon",
    "Tooltip": "Toggle Teams microphone mute via MQTT (teams-for-linux)",
    "Controllers": ["Keypad"],
    "PropertyInspectorPath": "pi/index.html",
    "States": [
      { "Title": "MIC", "FontSize": 18, "TitleColor": "#FFFFFF" },
      { "Title": "MUTED", "FontSize": 18, "TitleColor": "#FF4040" }
    ]
  }]
}
```

(Exact CodePaths key names verified against OpenDeck's manifest handling
during implementation; the per-target mechanism is already proven by the
current nix module.)

## Build, CI, release

- Static binaries: `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`
  via `cargo-zigbuild` in CI. Plain-TCP MQTT only → no TLS/openssl, so static
  linking is trivial.
- One universal zip containing both binaries; manifest `CodePaths` selects.
- `ci.yml` (push/PR): `cargo fmt --check`, `cargo clippy -- -D warnings`,
  `cargo test`, build both targets, assemble `.sdPlugin` dir, upload as
  workflow artifact (PRs produce installable test builds).
- `release.yml` (tag `v*`): same build, zip as
  `com.geoffdavis.teamsforlinux.sdPlugin.zip`, attach to GitHub Release.
  Manual tagging; release automation deferred.
- Icons are committed PNGs; `scripts/generate-icons.sh` regenerates them
  (imagemagick, ported 1:1 from the nix `drawIcon` derivations).

## Flake outputs

- `packages.<system>.default` — assembled `.sdPlugin` directory derivation
  (native rustPlatform build per system; no cross in nix).
- `homeManagerModules.default`:
  - installs the plugin into
    `~/.config/opendeck/plugins/com.geoffdavis.teamsforlinux.sdPlugin/`
  - `programs.opendeck-teams-for-linux.settings` option set { host, port,
    username, passwordFile, topicPrefix, topic overrides } → writes
    `~/.config/opendeck-teams-for-linux/config.toml`
- `devShells.default` — rustc/cargo, rust-analyzer, clippy, rustfmt,
  imagemagick. Paired with `.envrc` (`use flake`) per the nix-direnv
  convention on birdrock (dev tooling stays out of the global environment).

## Migration plan (consumer repo)

1. Add `opendeck-teams-for-linux` as a flake input; import the HM module;
   set `settings` (password via the existing op-CLI activation pattern, now
   writing the file `passwordFile` points at).
2. Delete from `modules/teams-for-linux.nix`: the plugin script, `writePython3Bin`
   wiring, icon derivations, plugin `xdg.configFile` entries, and the
   `mqtt.env` block. Delete `modules/teams-mute-plugin.py`.
3. Keep in the consumer repo: mosquitto service + passwordfile,
   teams-for-linux `config.json` generation, dev-overlay. These are host
   concerns.
4. Re-add the button once in OpenDeck (UUID changed from
   `com.gdavis.teamsmute`).

## Error handling

- MQTT: retry-forever with 5 s backoff; disconnects logged at warn.
- WebSocket (OpenDeck): errors logged; process exit lets OpenDeck restart the
  plugin.
- Unconfigured: `SETUP` display state; key presses no-op.
- Not in call: `OFF` display state; key presses no-op (matches Python guard).
- Malformed MQTT payloads: ignored.

## Testing

- Unit tests (pure, no I/O):
  - state machine: every payload/topic transition above, including
    "off forces in_call=false", "call end resets muted", initial-unknown
    behavior, and display mapping (incl. `SETUP`).
  - settings resolution: defaults < file < PI layering, `password_file`
    precedence, configured/unconfigured determination.
- Protocol-level integration test (`tests/protocol.rs`): the real plugin
  binary against a mock OpenDeck WebSocket server and a throwaway mosquitto
  broker — registration, willAppear → display pushes, MQTT-driven title
  transitions, keyDown → toggle-mute publish, not-in-call alert. Runs on any
  machine and in CI; needed because the dev laptop has no OpenDeck install
  (and OpenDeck is not in nixpkgs).
- Final hardware smoke test against the real stack (mosquitto +
  teams-for-linux + OpenDeck + deck) happens on the work machine post-release.
- CI runs fmt/clippy/test on every PR.

## Future ideas (explicitly out of scope for v1)

- OpenDeck store listing once teams-for-linux PR #2608 is merged.
- TLS/remote broker support in a later release, via `rumqttc`'s `rustls`
  feature (pure Rust — preserves static musl builds). Adds `mqtts://`-style
  config (broker scheme or TLS toggle + optional CA path) to file and PI.
- Optional use of the explicit `mute`/`unmute` commands (with `force`) instead
  of `toggle-mute`.
- Additional actions: camera toggle, in-call indicator, presence display.
