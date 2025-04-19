# killa 🔪

Killa is an opinionated GUI process monitor written in Rust, targeted for Linux desktops.
Works on Wayland and X.

tech: [Iced][iced] (frontend) + [Bottom][bottom] (backend)

# Why another system monitor?

I wanted a desktop system monitor with looks and UX of [gnome-system-monitor][gsm], but with faster statup times. I couldn't find any other UI system monitor that I like (including many terminal ones like atop, btop, htop, glances, etc.). So I've wrote my own.

Gnome System Monitor startup times are around ~3-5s for me on NixOS, while killa is ~500ms.

# Status

Very barebones for now, but usable (I main it).
Many features are still missing, especially killing processes.
But most of the ones I need are already implemented.

# Features

- Instant startup time (~500ms on my system)
- Shows a list of processes, sorted by CPU usage, refreshed every 1s
- Ctrl+F to search and filter processes
  - search is case-insensitive, terms split by spaces, exlude a term with `-` prefix
- Shows total memory usage %

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
[iced]: https://github.com/iced-rs/iced
[bottom]: https://github.com/ClementTsang/bottom
[gsm]: https://apps.gnome.org/SystemMonitor/