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
