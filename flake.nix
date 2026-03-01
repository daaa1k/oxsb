{
  description = "oxsb — Cross-platform sandbox wrapper";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    let
      # Home Manager module — system-agnostic, exported at the top level.
      #
      # Usage in a Home Manager configuration:
      #
      #   inputs.oxsb.url = "github:daaa1k/oxsb";
      #
      #   { inputs, ... }: {
      #     imports = [ inputs.oxsb.homeManagerModules.default ];
      #     programs.oxsb = {
      #       enable = true;
      #       settings = {
      #         backend.auto = true;
      #         write_allow = [
      #           { path = "$HOME/.config"; }
      #           { path = "/tmp"; }
      #         ];
      #         env.set.IN_SANDBOX = "1";
      #       };
      #     };
      #   }
      hmModule = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.oxsb;
          yamlFormat = pkgs.formats.yaml { };
        in
        {
          options.programs.oxsb = {
            enable = lib.mkEnableOption "oxsb cross-platform sandbox wrapper";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              defaultText = lib.literalExpression "oxsb.packages.\${pkgs.system}.default";
              description = "The oxsb package to install.";
            };

            settings = lib.mkOption {
              type = yamlFormat.type;
              default = { };
              description = ''
                Configuration for oxsb written to
                {file}`$XDG_CONFIG_HOME/oxsb/config.yaml`.

                Top-level keys mirror the YAML schema:
                `backend`, `write_allow`, `bubblewrap`, `seatbelt`, `env`.
                See {file}`examples/config_default.yaml` in the oxsb repository
                for a fully-annotated reference.
              '';
              example = lib.literalExpression ''
                {
                  backend.auto = true;
                  write_allow = [
                    { path = "$HOME/.config"; }
                    { path = "$HOME/.cache"; }
                    { path = "$HOME/.local/share"; }
                    { path = "/tmp"; }
                  ];
                  env.set.IN_SANDBOX = "1";
                }
              '';
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [ cfg.package ];

            xdg.configFile."oxsb/config.yaml" = lib.mkIf (cfg.settings != { }) {
              source = yamlFormat.generate "oxsb-config.yaml" cfg.settings;
            };
          };
        };
    in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        # Arguments shared between dependency pre-build and the final build.
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          # libiconv is required on macOS (see build.rs).
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };

        # Pre-build only the dependencies to maximise cache reuse.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        oxsb = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });
      in
      {
        # --- packages ---------------------------------------------------
        packages = {
          default = oxsb;
          inherit oxsb;
        };

        # --- checks (run by `nix flake check`) --------------------------
        checks = {
          # Build the package itself.
          inherit oxsb;

          # Run clippy with --deny warnings.
          oxsb-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Run the test suite.
          oxsb-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        # --- devShell ---------------------------------------------------
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.rust-analyzer
            pkgs.rustfmt
          ];
        };
      }
    ) // {
      # --- Home Manager module (system-agnostic) ----------------------
      homeManagerModules.default = hmModule;
    };
}
