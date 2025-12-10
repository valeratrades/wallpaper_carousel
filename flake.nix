{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    pre-commit-hooks.url = "github:cachix/git-hooks.nix";
    v-utils.url = "github:valeratrades/.github";
    wrap-it = {
      url = "github:valeratrades/wrap-it/cf3de8ced50c353ccfd534f3bb1ae9f6d5a04788";
      flake = false;
    };
  };
  outputs = { self, nixpkgs, rust-overlay, flake-utils, pre-commit-hooks, v-utils, wrap-it }:
    flake-utils.lib.eachDefaultSystem
      (
        system:
        let
          overlays = [ (import rust-overlay) ];
          pkgs = import nixpkgs {
            inherit system overlays;
            allowUnfree = true;
          };
          #NB: can't load rust-bin from nightly.latest, as there are week guarantees of which components will be available on each day.
          rust = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
            extensions = [ "rust-src" "rust-analyzer" "rust-docs" "rustc-codegen-cranelift-preview" ];
          });
          pre-commit-check = pre-commit-hooks.lib.${system}.run (v-utils.files.preCommit { inherit pkgs; });
          manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
          pname = manifest.name;
          stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;

          workflowContents = v-utils.ci {
            inherit pkgs;
            lastSupportedVersion = "nightly-2025-11-18";
            jobsErrors = [ "rust-tests" ];
            jobsWarnings = [ "rust-doc" "rust-clippy" "rust-machete" "rust-sorted" "rust-sorted-derives" "tokei" ];
            jobsOther = [ "loc-badge" ];
          };
          readme = v-utils.readme-fw {
            inherit pkgs pname;
            lastSupportedVersion = "nightly-1.93";
            rootDir = ./.;
            licenses = [{ name = "Blue Oak 1.0.0"; outPath = "LICENSE"; }];
            badges = [ "msrv" "crates_io" "docs_rs" "loc" "ci" ];
          };
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
                  mkdir -p .cache/typst/packages/preview/wrap-it
                  ln -s ${wrap-it} .cache/typst/packages/preview/wrap-it/0.1.1
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

                  # Include vision document (pre-built output)
                  mkdir -p $out/share/vision
                  cp ${visionDocument}/output.png $out/share/vision/vision.png
                  cp ${visionDocument}/output.pdf $out/share/vision/vision.pdf

                  # Include vision source for runtime regeneration
                  mkdir -p $out/share/vision/src_typ
                  cp -r ${./src_typ}/* $out/share/vision/src_typ/
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
                + workflowContents.shellHook
                + ''
                  cp -f ${v-utils.files.licenses.blue_oak} ./LICENSE

                  cargo -Zscript -q ${v-utils.hooks.appendCustom} ./.git/hooks/pre-commit
                  cp -f ${(v-utils.hooks.preCommit) { inherit pkgs pname; }} ./.git/hooks/custom.sh
                  cp -f ${(v-utils.hooks.treefmt) { inherit pkgs; }} ./.treefmt.toml

                  mkdir -p ./.cargo
                  cp -f ${ (v-utils.files.gitignore { inherit pkgs; langs = [ "rs" ]; }) } ./.gitignore
                  cp -f ${(v-utils.files.rust.clippy { inherit pkgs; })} ./.cargo/.clippy.toml
                  cp -f ${(v-utils.files.rust.config { inherit pkgs; })} ./.cargo/config.toml
                  cp -f ${(v-utils.files.rust.rustfmt { inherit pkgs; })} ./.rustfmt.toml

                  cp -f ${readme} ./README.md

                  mkdir -p ./assets
                  cp -f ${pkgs.dejavu_fonts}/share/fonts/truetype/DejaVuSansMono.ttf ./assets/DejaVuSansMono.ttf

                  alias qr="./target/debug/${pname}"
                '';

              packages = [
                mold-wrapped
                openssl
                pkg-config
                rust
                dejavu_fonts
              ] ++ pre-commit-check.enabledPackages;

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
