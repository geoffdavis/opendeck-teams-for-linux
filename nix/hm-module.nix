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
