# killa 🔪

Killa is a GUI process monitor written in Rust, targeted for Linux desktops.
The primary goals are to be fast, be simple, and keyboard-driven, while being usable with mouse too.
Works on Wayland and X.

tech: [Iced][iced] (frontend) + [Bottom][bottom] (backend)

# Why another system monitor?

I wanted a desktop system monitor with looks and UX of [gnome-system-monitor][gsm], but with faster statup times.
I couldn't find any other UI system monitor that I like (including many terminal ones like atop, btop, htop, glances, etc.).
So I've wrote my own.

Gnome System Monitor startup times are around ~3-5s for me on NixOS, while killa is ~500ms.

# Status

All essential features are implemented.
It is usable to the point that 99% of the time I no longer need to use other process monitors.
No more significant features are planned at this point in time.

# Features

- Instant startup time (~500ms on my system)
- Shows a list of processes, sorted by CPU usage, refreshed every 1s
- Shows total memory usage %
- Advanced searching:
  - Ctrl+F to focus search field.
  - Case-insensitive.
  - Terms split by spaces.
  - Prefix with `-` to revert the filter.
  - Search in specific by column using prefixes: `name`, `pid`, `cmd`, `any` (default). Examples:
      - `name:nix`
      - `pid:1`
      - `cmd:chrome`
      - `any:test:123` (searches for literal "test:123")
      - can be combined with `-` like this: `-pid:1`
- Allows killing processes
  (you can press Esc at any step to cancel)
  1. Filter/search processes (at least 3 chars)
  2. Ctrl+J to freeze. This stops the search results from updating.
  3. Ctrl+K to stage SIGTERM / Ctrl+Shift+K to stage SIGKILL.
  4. Press Enter to actually send the staged signal to all filtered processes.

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