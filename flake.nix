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

        # mosquitto must be in PATH for the protocol integration test
        nativeBuildInputs = [ pkgs.mosquitto ];

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
