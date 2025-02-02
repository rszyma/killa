# killa 🔪

Killa is a simple GUI process monitor written in Rust, targeted for Linux desktops.
Works on Wayland and X.

It's inspired by gnome-system-monitor looks. But unlike gnome-system-monitor,
it's aiming to have instant startup times, and high ergonomics with just keyboard.

tech: Iced (frontend) + Bottom (backend)

# Features

- show list of processes, sorted by CPU usage, refreshed every 1s
- show total memory usage %
- instant startup time (~500ms on my system)
- ctrl+f to search and filter processes

# Installation

The primary way to install is to use Nix with flakes:

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager.url = "github:nix-community/home-manager";
    killa.url = "github:rszyma/killa";
  };

  outputs = { nixpkgs, ... } @ inputs: {
    # set up for NixOS
    nixosConfigurations.HOSTNAME = nixpkgs.lib.nixosSystem {
      specialArgs = { inherit inputs; };
      modules = [ ./configuration.nix ];
    };

    # or for Home Manager
    homeConfigurations.HOSTNAME = inputs.home-manager.lib.homeManagerConfiguration {
      pkgs = import nixpkgs { inherit system; };
      extraSpecialArgs = { inherit inputs; };
      modules = [ ./home.nix ];
    }
  }
}
```
Then, add the package:

```nix
{ pkgs, inputs, ... }:
{
  environment.systemPackages = [ # or home.packages
    inputs.killa.packages.${pkgs.system}.default
  ];
}
```