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
        mkScript = name: text: (pkgs.writeShellScriptBin name text);
        devshellScripts = [
          (mkScript "f" ''cargo run'')
          (mkScript "fr" ''cargo run --release'')
        ];
      in
      {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = with pkgs; [
            # (rust-bin.stable."1.73.0".default.override toolchainOverride)
            # (rust-bin.stable.latest.default.override toolchainOverride)
            (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override toolchainOverride))
            cmake
            pkg-config
            expat # xml parser
          ] ++ devshellScripts;
          buildInputs = with pkgs; [
            fontconfig
            freetype
            libGL
            # xorg
            libxkbcommon
            vulkan-loader
            wayland
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
