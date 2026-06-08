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

```json
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
```

The default topics (`microphone`, `microphone/control`, `in-call`, `command`
under the `teams` prefix) match this plugin's defaults — if you customize
`mqtt.mediaTopics`, mirror the values in the plugin settings.

> **Note:** the `microphone/control` topic (`muted`/`unmuted`/`off`) is not in
> stock teams-for-linux yet — it's added by upstream PR
> [IsmaelMartinez/teams-for-linux#2608](https://github.com/IsmaelMartinez/teams-for-linux/pull/2608).
> Until that merges, run a build that includes it; otherwise the plugin derives
> mute state from the standard `microphone` topic
> (`speaking`/`silent`/`muted`/`off`), which works without the PR.

## Configure the plugin

Open the button's property inspector in OpenDeck and fill in the broker
credentials. Empty fields inherit from the optional config file or built-in
defaults (host `127.0.0.1`, port `1883`, prefix `teams`).

### Declarative config file (optional)

`~/.config/opendeck-teams-for-linux/config.toml` — every key optional:

```toml
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
```

Resolution order: built-in defaults < config file < non-empty property
inspector fields.

> **Security note:** values entered in the property inspector (including the
> password) are stored as plain JSON in OpenDeck's settings store, equivalent
> in trust to a `0600` config file. The intended model is a localhost-only
> broker; TLS/remote brokers are planned for a later release.

## Nix / home-manager

```nix
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
```

The module installs the plugin into `~/.config/opendeck/plugins/` and writes
the config file; populate `password_file` outside the Nix store (e.g. an
activation script reading your secrets manager).

## Development

```bash
direnv allow          # rust toolchain, python+pillow, mosquitto via nix-direnv
cargo test            # state machine + settings resolution + e2e protocol test
cargo build --release
scripts/assemble-plugin.sh   # -> dist/com.geoffdavis.teamsforlinux.sdPlugin
scripts/generate-icons.sh    # regenerate committed icons
```

How it works: a background task subscribes to the state topics and folds
payloads into a `TeamsState { muted, camera_on, in_call }` state machine; a watch
channel fans changes out to every visible button instance. Each action publishes
its command (e.g. `{"action":"toggle-mute"}` or `{"action":"toggle-video"}`) to
the command topic, guarded by in-call state. With upstream PR #2608,
teams-for-linux's `microphone/control` topic (`muted`/`unmuted`/`off`) is the
authoritative mute signal; without it the plugin derives mute state from the
standard `microphone` topic (`speaking`/`silent`/`muted`/`off`).

## License

[MIT](LICENSE)
