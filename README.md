![Pinnacle banner](/assets/pinnacle_banner_dark.png)

<div align="center">

[![Discord](https://img.shields.io/discord/1223351743522537565?style=for-the-badge&logo=discord&logoColor=white&label=Discord&labelColor=%235865F2&color=%231825A2)](https://discord.gg/JhpKtU2aMA)
[![Matrix](https://img.shields.io/matrix/pinnacle%3Amatrix.org?style=for-the-badge&logo=matrix&logoColor=white&label=Matrix&labelColor=black&color=gray)](https://matrix.to/#/#pinnacle:matrix.org)

</div>

![image](https://github.com/pinnacle-comp/pinnacle/assets/120758733/a0e9ce93-30bb-4359-9b61-78ad4c4134d9)

# Table of Contents
- [Info](#info)
    - [What is Pinnacle?](#what-is-pinnacle)
    - [Features](#features)
    - [Roadmap](#roadmap)
- [Dependencies](#dependencies)
- [Building](#building)
- [Running](#running)
- [Configuration](#configuration)
    - [Out-of-the-box configurations](#out-of-the-box-configurations)
    - [Custom configuration](#custom-configuration)
        - [Generating a config](#generating-a-config)
    - [More on configuration](#more-on-configuration)
        - [The `metaconfig.toml` file](#the-metaconfigtoml-file)
    - [Lua Language Server completion](#lua-language-server-completion)
    - [API references](#api-references)
- [Controls](#controls)
- [Feature Requests, Bug Reports, Contributions, and Questions](#feature-requests-bug-reports-contributions-and-questions)
- [Changelog](#changelog)

# Info
### What is Pinnacle?
Pinnacle is a Wayland compositor built in Rust using [Smithay](https://github.com/Smithay/smithay).
It's my attempt at creating something like [AwesomeWM](https://github.com/awesomeWM/awesome)
for Wayland.

Pinnacle comes integrated with [Snowcap](https://github.com/pinnacle-comp/snowcap), a
very, *very* WIP widget system. Currently it's only being used for the builtin quit prompt and keybind overlay.
In the future, Snowcap will be used for everything Awesome uses its widget system for: a taskbar, system tray, etc.

### Features
- Tag system
- Customizable layouts, including most of the ones from Awesome
- (Scuffed) XWayland support
- wlr-layer-shell support
- Configurable in Lua or Rust
- wlr-screencopy support
- A really *really* WIP widget system
- Is very cool :thumbsup:

### Roadmap
- See [#142](https://github.com/pinnacle-comp/pinnacle/issues/142)

# Dependencies
You will need:

- [Rust](https://www.rust-lang.org/) 1.76 or newer
- The following external dependencies:
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

The following are optional dependencies:

- [`just`](https://github.com/casey/just) to automate installation of libraries and files
- The following are required to use the Lua API:
    - `just` as mentioned above
    - [`lua`](https://www.lua.org/) 5.2 or newer
    - [`luarocks`](https://luarocks.org/) for API installation
        - You must run `eval $(luarocks path --lua-version <your-lua-version>)` so that your config can find the API
          library files. It is recommended to place this command in your shell's startup script.

- Arch and derivatives:
    ```sh
    sudo pacman -S wayland libxkbcommon libinput mesa seatd systemd-libs libdisplay-info xorg-xwayland protobuf
    # And optionally
    sudo pacman -S just lua luarocks
    ```
- Debian and derivatives:
    ```sh
    sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgbm-dev libseat-dev libsystemd-dev protobuf-compiler xwayland libegl-dev libdisplay-info-dev
    # And optionally
    sudo apt install just lua5.4 luarocks
    ```
    - Note: `just` is only available in apt from Debian 13.
- Nix and NixOS:
    - Use the provided [`flake.nix`](flake.nix) with a devShell. It also
      includes the other tools needed for the build and sets up the
      `LD_LIBRARY_PATH` so the dynamically loaded libraries are found.
      > Luarocks currently doesn't install the Lua library and its dependencies due to openssh directory
      > shenanigans. Fix soon, hopefully. In the meantime you can use the Rust API.

TODO: other distros

# Building

Clone this repository:
```sh
git clone https://github.com/pinnacle-comp/pinnacle
```

> [!NOTE]
> For all following `cargo`/`just` commands, if you would like to build without Snowcap integration,
> run with `--no-default-features`.

Build the project with:
```sh
cargo build [--release]
```

To additionally install the default configs, protobuf definitions, and Lua API, run:
```sh
just install build [--release] # Order matters, put build/run/test last to pass through arguments
```

# Running
> [!TIP]
> Before running, read the information in [Configuration](#configuration).

> [!IMPORTANT]
> If you are going to use a Lua config, you must run `just install` to install the protobuf definitions
> and Lua library.

After building, run the executable located in either:
```sh
./target/debug/pinnacle     # without --release
./target/release/pinnacle   # with --release
```

> [!IMPORTANT]
> When compiling with Snowcap integration, if you do not have Vulkan set up properly,
> Pinnacle will crash on startup.
>
> For those using Nix outside of NixOS, you will need to run the built binary
> with [nixGL](https://github.com/nix-community/nixGL) using *both* GL and Vulkan wrappers, nested inside one another:
> ```
> nix run --impure github:nix-community/nixGL -- nix run --impure github:nix-community/nixGL#nixVulkanIntel -- ./target/debug/pinnacle
> ```

Or, run the project directly with 
```sh
cargo run [--release]

# With installation:
just install run [--release]
```

See flags Pinnacle accepts by running `cargo run -- --help` (or `-h`).

# Configuration
Pinnacle is configured in your choice of Lua or Rust.

## Out-of-the-box configurations
Pinnacle embeds the default Rust config into the binary. If you would like to use
the Lua or Rust default configs standalone, run one of the following in the crate root:

```sh
# For a Lua configuration
just install run -- -c ./api/lua/examples/default

# For a Rust configuration
cargo run -- -c ./api/rust/examples/default_config
```

When running a Rust config without compiled Snowcap integration,
use the following directory instead (Lua users can use the same directory):
```sh
cargo run -- -c ./api/rust/examples/default_config_no_snowcap
```

## Custom configuration

> [!IMPORTANT]
> Pinnacle is under development, and there *will* be major breaking changes to these APIs
> until I release version 0.1, at which point there will be an API stability spec in place.

### Generating a config

Run the following command to open up the interactive config generator:
```sh
just install-configs run -- config gen
```

This will prompt you to choose a language (Lua or Rust) and directory to put the config in.
It will then generate a config at that directory. If Lua is chosen and there are conflicting
files in the directory, the generator will prompt to rename them to a backup before continuing.
If Rust is chosen, the directory must be manually emptied to continue.

Note that this currently copies default configs *with* Snowcap integration.

Run `cargo run -- config gen --help` for information on the command.

## More on configuration
Pinnacle is configured mostly at runtime through IPC using [gRPC](https://grpc.io/). This is done through
configuration clients that use the [Lua](api/lua) and [Rust](api/rust) APIs.

As the compositor has no direct integration with these clients, it must know what it needs to run
through a separate file, aptly called the `metaconfig.toml` file.

To start a config, Pinnacle will search for a `metaconfig.toml` file in the first directory
that exists from the following:

1. The directory passed in through `--config-dir`/`-c`
2. `$PINNACLE_CONFIG_DIR`
3. `$XDG_CONFIG_HOME/pinnacle`
4. `~/.config/pinnacle` if `$XDG_CONFIG_HOME` is not defined

If there is no `metaconfig.toml` file in that directory, Pinnacle will start the embedded
Rust config.

Additionally, if your config crashes, Pinnacle will also start the embedded Rust config.

> [!NOTE]
> If you are using a Lua config and have not run `eval $(luarocks path --lua-version <your-lua-version>)`,
> Pinnacle will fallback to the embedded Rust config.

### The `metaconfig.toml` file
A `metaconfig.toml` file must contain the following entries:
- `command`: An array denoting the program and arguments Pinnacle will run to start a config.
- `reload_keybind`: A table denoting a keybind that Pinnacle will hardcode to restart your config.
- `kill_keybind`: A table denoting a keybind that Pinnacle will hardcode to quit the compositor.
    - The two keybinds above prevent you from getting locked in the compositor if the default config fails to start.

It also has the following optional entries:
- `socket_dir`: A directory that Pinnacle will place its IPC socket in (this defaults to `$XDG_RUNTIME_DIR`,
  falling back to `/tmp` if that doesn't exist).
- `[envs]`: A table of environment variables that Pinnacle will start the config with.

For the specifics, see the default [`metaconfig.toml`](api/lua/examples/default/metaconfig.toml) file.

## Lua Language Server completion
A [`.luarc.json`](api/lua/examples/default/.luarc.json) file is included with the default Lua config
and will set the correct workspace library files for use with the
[Lua language server](https://github.com/LuaLS/lua-language-server).

## API references
<b>Lua: https://pinnacle-comp.github.io/lua-reference/.<br>
Rust: https://pinnacle-comp.github.io/rust-reference/main.</b>

> Documentation for the Rust API can be reached by replacing `main` with the branch you want.
> Other branches for Lua soon<sup>tm</sup>

# Controls

The following are the default controls, mirroring Awesome's defaults.

<kbd>Mod</kbd> is <kbd>Super</kbd> when running in a tty and <kbd>Alt</kbd> when running as a nested window.

| Binding                                                                      | Action                            |
|------------------------------------------------------------------------------|-----------------------------------|
| <kbd>Mod</kbd> + <kbd>s</kbd>                                                | Show the keybind overlay          |
| <kbd>Mod</kbd> + <kbd>Mouse left drag</kbd>                                  | Move window                       |
| <kbd>Mod</kbd> + <kbd>Mouse right drag</kbd>                                 | Resize window                     |
| <kbd>Mod</kbd><kbd>Shift</kbd> + <kbd>q</kbd>                                | Quit Pinnacle                     |
| <kbd>Mod</kbd><kbd>Ctrl</kbd> + <kbd>r</kbd>                                 | Reload the config                 |
| <kbd>Mod</kbd><kbd>Shift</kbd> + <kbd>c</kbd>                                | Close window                      |
| <kbd>Mod</kbd> + <kbd>Return</kbd>                                           | Spawn [Alacritty](https://github.com/alacritty/alacritty) (you can change this in the config) |
| <kbd>Mod</kbd><kbd>Ctrl</kbd> + <kbd>Space</kbd>                             | Toggle floating                   |
| <kbd>Mod</kbd> + <kbd>f</kbd>                                                | Toggle fullscreen                 |
| <kbd>Mod</kbd> + <kbd>m</kbd>                                                | Toggle maximized                  |
| <kbd>Mod</kbd> + <kbd>Space</kbd>                                            | Cycle to the next layout          |
| <kbd>Mod</kbd><kbd>Shift</kbd> + <kbd>Space</kbd>                            | Cycle to the previous layout      |
| <kbd>Mod</kbd> + <kbd>1</kbd> to <kbd>5</kbd>                                | Switch to tag `1` to `5`          |
| <kbd>Mod</kbd><kbd>Ctrl</kbd> + <kbd>1</kbd> to <kbd>5</kbd>                 | Toggle tag `1` to `5`             |
| <kbd>Mod</kbd><kbd>Shift</kbd> + <kbd>1</kbd> to <kbd>5</kbd>                | Move a window to tag `1` to `5`   |
| <kbd>Mod</kbd><kbd>Ctrl</kbd><kbd>Shift</kbd> + <kbd>1</kbd> to <kbd>5</kbd> | Toggle tag `1` to `5` on a window |

# Feature Requests, Bug Reports, Contributions, and Questions
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

# Changelog
See [`CHANGELOG.md`](CHANGELOG.md).

# With Special Thanks To
- [Smithay](https://github.com/Smithay/smithay): For being a great compositor library and also allowing me not to deal with all the graphics stuff I still don't understand
- [Niri](https://github.com/YaLTeR/niri): For all that rendering and protocol stuff I, ahem, *took inspiration* from
