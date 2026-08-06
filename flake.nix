{
  inputs = {
    v_flakes.url = "github:valeratrades/v_flakes?ref=v1.6";
    wrap-it = {
      url = "github:valeratrades/wrap-it/cf3de8ced50c353ccfd534f3bb1ae9f6d5a04788";
      flake = false;
    };
  };
  outputs = { self, v_flakes, wrap-it }:
    let
      inherit (v_flakes) flake-utils pre-commit-hooks;
    in
    flake-utils.lib.eachDefaultSystem
      (
        system:
        let
          pkgs = import v_flakes.default_nixpkgs { inherit system; config.allowUnfree = true; };
          rust = v_flakes.rs.default_nightly system;
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v_flakes.files.preCommit { inherit pkgs; });
          manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
          pname = manifest.name;
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          rs = v_flakes.rs {
            inherit pkgs rust;
          };
          github = v_flakes.github {
            inherit pkgs pname rs;
            enable = true;
            lastSupportedVersion = "nightly-2026-06-29";
            jobs.default = true;
          };
          readme = v_flakes.readme-fw {
            inherit pkgs pname;
            lastSupportedVersion = "nightly-1.93";
            rootDir = ./.;
            licenses = [{ license = v_flakes.files.licenses.nsfw; }];
            badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ];
          };
          combined = v_flakes.utils.combine { inherit rust; modules = [ readme github rs ]; };
        in
        {
          packages =
            let
              rustc = rust;
              cargo = rust;
              rustPlatform = pkgs.makeRustPlatform {
                inherit rustc cargo stdenv;
              };

              visionDocument = pkgs.stdenvNoCC.mkDerivation {
                name = "vision-document";
                src = ./.;

                nativeBuildInputs = [ pkgs.typst ];

                buildPhase = ''
                  mkdir -p .cache/typst/packages/local/wrap-it
                  ln -sfn ${wrap-it} .cache/typst/packages/local/wrap-it/0.1.1
                  export XDG_CACHE_HOME=$(pwd)/.cache
                  typst compile src_typ/vision.typ output.pdf
                  typst compile --format png src_typ/vision.typ output{n}.png
                  if [ -f output2.png ]; then
                    echo "Error: More than 1 page generated. Vision document must be single-page."
                    exit 1
                  fi
                  mv output1.png output.png
                '';

                installPhase = ''
                  mkdir -p $out
                  cp output.pdf $out/
                  cp output.png $out/
                '';
              };
            in
            {
              default = rustPlatform.buildRustPackage rec {
                inherit pname;
                version = manifest.version;

                buildInputs = with pkgs; [
                  openssl.dev
                  dejavu_fonts
                ];
                nativeBuildInputs = with pkgs; [ pkg-config ];

                cargoLock.lockFile = ./Cargo.lock;
                src = pkgs.lib.cleanSource ./.;

                # Make DejaVu fonts available at runtime
                postInstall = ''
                  mkdir -p $out/share/fonts
                  ln -s ${pkgs.dejavu_fonts}/share/fonts/truetype $out/share/fonts/truetype
                '';

                # Set FONTCONFIG_PATH to include our fonts
                makeWrapperArgs = [
                  "--prefix"
                  "FONTCONFIG_PATH"
                  ":"
                  "$out/share/fonts"
                ];
              };

              vision = visionDocument;
            };

          devShells.default =
            with pkgs;
            mkShell {
              inherit stdenv;
              shellHook =
                pre-commit-check.shellHook
                + combined.shellHook
                + ''
                  mkdir -p ./assets
                  cp -f ${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSansMono.ttf ./assets/DejaVuSansMono.ttf
                ''
              ;

              packages = [
                mold
                openssl
                pkg-config
                rust
                dejavu_fonts
              ] ++ pre-commit-check.enabledPackages ++ combined.enabledPackages;

              env.RUST_BACKTRACE = 1;
              env.RUST_LIB_BACKTRACE = 0;
            };
        }
      ) // {
      homeManagerModules.wallpaper-carousel = { config, lib, pkgs, ... }:
        let
          inherit (lib) mkEnableOption mkOption mkIf;
          inherit (lib.types) package;
          cfg = config.wallpaper-carousel;
        in
        {
          options.wallpaper-carousel = {
            enable = mkEnableOption "wallpaper carousel hourly extend";

            package = mkOption {
              type = package;
              description = "The wallpaper_carousel package to use.";
            };
          };

          config = mkIf cfg.enable {
            systemd.user.timers.wallpaper-extend = {
              Unit = {
                Description = "Timer to run wallpaper extend every hour";
              };

              Timer = {
                OnBootSec = "1h";
                OnUnitActiveSec = "1h";
                Persistent = true;
              };

              Install = {
                WantedBy = [ "timers.target" ];
              };
            };

            systemd.user.services.wallpaper-extend = {
              Unit = {
                Description = "Extend wallpaper with text overlays";
              };

              Service = {
                Type = "oneshot";
                ExecStart = ''
                  /bin/sh -c 'if ! ${cfg.package}/bin/wallpaper_carousel extend 2>&1 | grep -q "No input file provided"; then exit 0; else echo "Warning: No cached input file, skipping wallpaper extend"; exit 0; fi'
                '';
              };
            };
          };
        };
    };
}
