![Pinnacle banner](/assets/pinnacle_banner_dark.png)

https://github.com/Ottatop/pinnacle/assets/120758733/c175ba80-9796-4759-92c3-1d7a6639b0c9

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
        - [Lua](#lua)
            - [Lua Language Server completion](#lua-language-server-completion)
        - [Rust](#rust)
        - [API References](#api-references)
- [Controls](#controls)
- [Feature Requests, Bug Reports, Contributions, and Questions](#feature-requests-bug-reports-contributions-and-questions)
- [Changelog](#changelog)

# Info
### What is Pinnacle?
Pinnacle is a Wayland compositor built in Rust using [Smithay](https://github.com/Smithay/smithay).
It's my attempt at creating something like [AwesomeWM](https://github.com/awesomeWM/awesome)
for Wayland.

It can be configured through either Lua or Rust.

> ### More video examples below!
> <details>
> 
> <summary>Click me</summary>
>
> All videos were recorded using [Screenkey](https://gitlab.com/screenkey/screenkey) and the Winit backend.
> 
> https://github.com/Ottatop/pinnacle/assets/120758733/5b6b224b-3031-4a1c-9375-1143f1bfc0e3
>
> https://github.com/Ottatop/pinnacle/assets/120758733/7a465983-2560-412e-9154-40b3dfd20488
>
> (This video is very crunchy in my attempts to get under the 10mb limit)
>
> </details>

### Features
- Tag system
- Left master stack, corner, dwindle, and spiral layouts from Awesome
- (Really scuffed) XWayland support
- Layer-shell support
- Configurable in Lua or Rust
- Is very cool :thumbsup:

### Roadmap
- TODO

# Dependencies
You will need:

- [Rust](https://www.rust-lang.org/) 1.74 or newer
    - If you want to use the Rust API, you will need Rust 1.75 or newer
- [Lua](https://www.lua.org/) 5.4 or newer, to use the Lua API
- Packages for [Smithay](https://github.com/Smithay/smithay):
  `libwayland libxkbcommon libudev libinput libgdm libseat`, as well as `xwayland`
    - Arch:
        ```sh
        sudo pacman -S wayland wayland-protocols libxkbcommon systemd-libs libinput mesa seatd xorg-xwayland
        ```
    - Debian:
        ```sh
        sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgdm-dev libseat-dev xwayland
        ```
    - NixOS: There is a really old [`shell.nix`](shell.nix) that may or may not work :skull:
- [protoc](https://grpc.io/docs/protoc-installation/), the Protocol Buffer Compiler, for configuration
    - Arch:
        ```sh
        sudo pacman -S protobuf
        ```
- [LuaRocks](https://luarocks.org/), the Lua package manager, to use the Lua API
    - Arch:
        ```sh
        sudo pacman -S luarocks
        ```
    - You must run `eval $(luarocks path --lua-version 5.4)` so that your config can find the API
      library files. It is recommended to place this command in your shell's startup script.

TODO: other distros

# Building
Build the project with:
```sh
cargo build [--release]
```

> [!NOTE]
> On build, [`build.rs`](build.rs) will: 
> - Copy Protobuf definition files to `$XDG_DATA_HOME/pinnacle/protobuf`
> - Copy the [default config](api/lua/examples/default) to `$XDG_DATA_HOME/pinnacle/default_config`
> - `cd` into [`api/lua`](api/lua) and run `luarocks make` to install the Lua library to `~/.luarocks/share/lua/5.4`

# Running
> [!IMPORTANT]
> Before running, read the information in [Configuration](#configuration).

After building, run the executable located in either:
```sh
./target/debug/pinnacle     # without --release
./target/release/pinnacle   # with --release
```

Or, run the project directly with 
```sh
cargo run [--release]
```

See flags you can pass in by running `cargo run -- --help` (or `-h`).

# Configuration
Pinnacle is configured in your choice of Lua or Rust.

## Out-of-the-box configurations
If you just want to test Pinnacle out without copying stuff to your config directory,
run one of the following in the crate root:
```sh
# For a Lua configuration
PINNACLE_CONFIG_DIR="./api/lua/examples/default" cargo run
# This should also have been copied to the directory below, so below will do the same
PINNACLE_CONFIG_DIR="~/.local/share/pinnacle/default_config" cargo run

# For a Rust configuration
PINNACLE_CONFIG_DIR="./api/rust/examples/default_config" cargo run
```

## Custom configuration

> [!IMPORTANT]
> Pinnacle is under development, and there *will* be major breaking changes to these APIs
> until I release version 0.1, at which point there will be an API stability spec in place.
>
> Until then, I recommend you either use the out-of-the-box configs above or prepare for
> your config to break every now and then.

Pinnacle will search for a `metaconfig.toml` file in the following directories, from top to bottom:
```sh
$PINNACLE_CONFIG_DIR
$XDG_CONFIG_HOME/pinnacle
~/.config/pinnacle # Only if $XDG_CONFIG_HOME is not defined
```

The `metaconfig.toml` file provides information on what config to run, kill and reload keybinds,
and any environment variables you want set. For more details, see the provided 
[`metaconfig.toml`](api/lua/examples/default/metaconfig.toml) file.

If no `metaconfig.toml` file is found, the default Lua config will be loaded.

### Lua
For custom configuration in Lua, copy the contents of `~/.local/share/pinnacle/default_config` to
`~/.config/pinnacle` (or `$XDG_CONFIG_HOME/pinnacle`):
```sh
mkdir ~/.config/pinnacle
cp -r ~/.local/share/pinnacle/default_config/. ~/.config/pinnacle
```
Note: there is a `.luarc.json` file that may not get copied if you do `cp <path>/* ~/.config/pinnacle`.
The above command takes that into account.

> If you rename `default_config.lua`, make sure `command` in your `metaconfig.toml` is updated to reflect that.
> If it isn't, the compositor will load the default config instead.

#### Lua Language Server completion
A [`.luarc.json`](api/lua/examples/default/.luarc.json) file is included with the default Lua config
and will set the correct workspace library files for use with the
[Lua language server](https://github.com/LuaLS/lua-language-server).

### Rust
If you want to use Rust to configure Pinnacle, follow these steps:
1. In `~/.config/pinnacle`, run `cargo init`.
2. In the `Cargo.toml` file, add the following under `[dependencies]`:
```toml
pinnacle-api = { git = "http://github.com/pinnacle-comp/pinnacle" }
```
3. Create the file `metaconfig.toml` at the root. Add the following to the file:
```toml
command = ["cargo", "run"]
reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }
```
4. Copy the [default config](api/rust/examples/default_config/main.rs) to `src/main.rs`.
5. Run Pinnacle! (You may want to run `cargo build` beforehand so you don't have to wait for your config to compile.)

### API References
<b>Lua: https://pinnacle-comp.github.io/lua-reference/main.<br>
Rust: https://pinnacle-comp.github.io/rust-reference/main.</b>

> Documentation for other branches can be reached by replacing `main` with the branch you want.

# Controls
The following are the default controls in the [`default_config`](api/lua/examples/default/default_config.lua).
| Binding                                      | Action                             |
|----------------------------------------------|------------------------------------|
| <kbd>Ctrl</kbd> + <kbd>Mouse left drag</kbd> | Move window                        |
| <kbd>Ctrl</kbd> + <kbd>Mouse right drag</kbd>| Resize window                      |
| <kbd>Ctrl</kbd><kbd>Alt</kbd> + <kbd>q</kbd> | Quit Pinnacle                      |
| <kbd>Ctrl</kbd><kbd>Alt</kbd> + <kbd>c</kbd> | Close window                       |
| <kbd>Ctrl</kbd> + <kbd>Return</kbd>          | Spawn [Alacritty](https://github.com/alacritty/alacritty) (you can change this in the config)|
| <kbd>Ctrl</kbd><kbd>Alt</kbd> + <kbd>Space</kbd> | Toggle between floating and tiled |
| <kbd>Ctrl</kbd> + <kbd>f</kbd>     | Toggle fullscreen        |
| <kbd>Ctrl</kbd> + <kbd>m</kbd>     | Toggle maximized         |
| <kbd>Ctrl</kbd> + <kbd>Space</kbd> | Cycle to the next layout |
| <kbd>Ctrl</kbd><kbd>Shift</kbd> + <kbd>Space</kbd>                           | Cycle to the previous layout      |
| <kbd>Ctrl</kbd> + <kbd>1</kbd> to <kbd>5</kbd>                               | Switch to tag `1` to `5`          |
| <kbd>Ctrl</kbd><kbd>Shift</kbd> + <kbd>1</kbd> to <kbd>5</kbd>               | Toggle tag `1` to `5`             |
| <kbd>Ctrl</kbd><kbd>Alt</kbd> + <kbd>1</kbd> to <kbd>5</kbd>                 | Move a window to tag `1` to `5`   |
| <kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>Shift</kbd> + <kbd>1</kbd> to <kbd>5</kbd> | Toggle tag `1` to `5` on a window |

# Feature Requests, Bug Reports, Contributions, and Questions
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

# Changelog
See [`CHANGELOG.md`](CHANGELOG.md).
