{
  # testing flake: nix develop --unset PATH

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay.url = "github:oxalica/rust-overlay/stable";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };
        # target = "wasm32-unknown-unknown";
        # targetUpperSnake = pkgs.lib.toUpper (builtins.replaceStrings [ "-" ] [ "_" ] target);
        toolchainOverride = {
          extensions = [ "rust-src" ];
          # targets = [ target ];
        };
      in
      {
        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            # (rust-bin.stable."1.73.0".default.override toolchainOverride)
            # (rust-bin.stable.latest.default.override toolchainOverride)
            (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override toolchainOverride))

            # deps
            cmake
            # ncurses
            pkg-config
            # scdoc
            expat
            fontconfig
            freetype
            libGL
            # xorg
            libxkbcommon
            vulkan-loader
            wayland
            # xdg-utils
          ];
          WINIT_UNIX_BACKEND = "wayland";
          # WGPU_POWER_PREF = "low";
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
          # RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
          # "CARGO_TARGET_${targetUpperSnake}_LINKER" = "${pkgs.lld_18}/bin/lld";
          # RUSTFLAGS = nixpkgs.lib.strings.concatStringsSep " " [ ];
          RUST_BACKTRACE = 1;
          RUST_LOG = "debug";
        };
      }
    );
}
