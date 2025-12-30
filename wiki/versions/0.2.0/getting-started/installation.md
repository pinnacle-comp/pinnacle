# Installation

## Distro-specific

It is recommended that you install Pinnacle using your distro's package manager.

::: tabs

== Arch (AUR)
```sh
yay -S pinnacle-comp

# Or latest commit
yay -S pinnacle-comp-git
```

:::

### NixOS
First, we need to set up Pinnacle as a session loadable by your display manager:

1. add the pinnacle overlay when importing nixpkgs:
    ```nix
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [
        inputs.pinnacle.overlays.default
        # your other overlays
      ];
    };
    ```
1. import the nixos module from the flake -- it looks like this if you're using a flake based config and the `nixosSystem` function:
    ```nix
      nixpkgs.lib.nixosSystem {
        inherit pkgs;
        modules = [
          inputs.pinnacle.nixosModules.default
          # your other configuration modules
        ];
      }
    ```
1. set up pinnacle as a wayland session:
    ```nix
      programs.pinnacle = {
        enable = true;
        # ensure portals are installed such that screenshots, screen recording, etc. work properly
        xdg-portals.enable = true;
        withUWSM = true;
      };

      # make sure to set the following if you're using auto-login
      services.displayManager = {
        defaultSession = "pinnacle-uwsm";
      };
  
      # not sure if the following is strictly necessary but you probably
      # want to make sure they're set somewhere:
      services.xserver.enable = true;
      programs.dconf.enable = true;
      security.polkit.enable = true;
      hardware.graphics.enable = true;
      # you should also make sure the xkb layout is configured
    ```

Then we need to configure your users to launch pinnacle properly:

1. load the home-manager module:
    ```nix
    {
      home-manager.sharedModules = [inputs.pinnacle.hmModules.default];
    }
    ```
1. set up the user service/targets in one of your home-manager user modules:
    ```nix
      wayland.windowManager.pinnacle = {
        enable = true;
        # if you're using rust -- if you're using lua, you'll need to set `wayland.windowManager.pinnacle.config.execCmd` to point to
        # your lua script.
        clientPackage = pkgs.pinnacle.buildRustConfig {
          name = "pinnacle-config";
          src = path/to/pinnacle/rust/config;
          version = "0.2.0";
        };
        systemd = {
          enable = true;
          # use UWSM instead
          useService = false;
          xdgAutostart = true;
        };
      };
      # make sure to start a bar and a launcher of some kind
    ```

## From source

Alternatively, you can build and install Pinnacle from source.

### Dependencies

To build the project, you will need Rust 1.88 or newer.

First, you will need the following dependencies:
- [`just`](https://github.com/casey/just)
- `libwayland`
- `libxkbcommon`
- `libudev`
- `libinput`
- `libgbm`
- `libseat`
- `libEGL`
- `libsystemd`
- `libdisplay-info` for monitor display information
- `xwayland` for Xwayland support
- [`protoc`](https://grpc.io/docs/protoc-installation/) for the API

To configure Pinnacle using Lua, you will also need:
- [`lua`](https://www.lua.org/) 5.2 or newer
- [`luarocks`](https://luarocks.org/) for API installation

### Building

Clone the repository.
```sh
git clone https://github.com/pinnacle-comp/pinnacle
```

To build Pinnacle, run `just build`. This passes through arguments to Cargo.
```sh
just build [Cargo arguments...]
```

To use the Lua API, you will also need to run `just install`. This will install the protobuf files
and Lua API locally.

> [!TIP]
> You can run multiple `just` recipes in one command, e.g.
> ```sh
> just install build
> ```
