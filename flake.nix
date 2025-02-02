{
  # testing flake: nix develop --unset PATH

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay/stable";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane, fenix, advisory-db }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        nativeBuildInputs = with pkgs; [
          cmake
          pkg-config
          expat # xml parser
          makeWrapper
        ];
        buildInputs = with pkgs; [
          # these 3 are absolutely necessary
          libxkbcommon
          vulkan-loader
          wayland

          # TODO: are these 3 needed?
          fontconfig
          freetype
          libGL

          # deps for X only.
          xorg.libX11
          xorg.libXi
          xorg.libXtst
        ];
        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";

        commonArgs = {
          inherit src;
          strictDeps = true;
          inherit nativeBuildInputs;
          inherit buildInputs;
          inherit LD_LIBRARY_PATH;
        };

        # toolchain = fenixPkgs.stable;
        # combinedToolchain = toolchain.completeToolchain;

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        killaCrate = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });

        wrappedKillaCrate = pkgs.symlinkJoin
          {
            name = "killa";
            paths = [ killaCrate ];
            buildInputs = [ pkgs.makeWrapper ];
            postBuild = ''
              wrapProgram $out/bin/killa --set LD_LIBRARY_PATH ${pkgs.lib.makeLibraryPath buildInputs} --set PATH $out/bin
            '';
          };

        mkScript = name: text: (pkgs.writeShellScriptBin name text);
        devshellScripts = [
          (mkScript "f" ''cargo run'')
          (mkScript "fr" ''cargo run --release'')
        ];
      in
      {
        packages = {
          default = wrappedKillaCrate;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          my-crate-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.default = flake-utils.lib.mkApp {
          drv = wrappedKillaCrate;
        };

        devShells.default = pkgs.mkShell {
          # checks = self.checks.${system};
          nativeBuildInputs = nativeBuildInputs ++ devshellScripts ++ (
            with pkgs;
            let
              toolchainOverride = { extensions = [ "rust-src" ]; };
            in
            [
              # (rust-bin.stable."1.73.0".default.override toolchainOverride)
              (rust-bin.stable.latest.default.override toolchainOverride)
              # (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override toolchainOverride))
            ]
          );
          inherit buildInputs;
          # WINIT_UNIX_BACKEND = "wayland";
          # WGPU_POWER_PREF = "low";
          inherit LD_LIBRARY_PATH;
          # RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
          # "CARGO_TARGET_${targetUpperSnake}_LINKER" = "${pkgs.lld_18}/bin/lld";
          # RUSTFLAGS = nixpkgs.lib.strings.concatStringsSep " " [ ];
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
        };

        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit killaCrate;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-crate-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          my-crate-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          my-crate-fmt = craneLib.cargoFmt {
            inherit src;
          };

          my-crate-toml-fmt = craneLib.taploFmt {
            src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
            # taplo arguments can be further customized below as needed
            # taploExtraArgs = "--config ./taplo.toml";
          };

          # Audit dependencies
          my-crate-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          my-crate-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `my-crate` if you do not want
          # the tests to run twice
          my-crate-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            cargoNextestPartitionsExtraArgs = "--no-tests=pass";
          });
        };

      }
    );
}
