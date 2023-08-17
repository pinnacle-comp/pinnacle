# <div align="center">Pinnacle</div>
<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="/assets/pinnacle_banner_dark.png">
        <source media="(prefers-color-scheme: light)" srcset="/assets/pinnacle_banner_light.png">
        <img alt="Pinnacle banner" src="/assets/pinnacle_banner_light.png">
    </picture>
</div>

## Info
### What is Pinnacle?
Pinnacle is a Wayland compositor built in Rust using [Smithay](https://github.com/Smithay/smithay).
It's my attempt at creating something like [AwesomeWM](https://github.com/awesomeWM/awesome)
for Wayland.

It sports high configurability through a (soon to be) extensive Lua API, with plans for a Rust API in the future.

Showcase/gallery soon:tm:

### Features
> This is a non-exhaustive list.
- [x] Winit backend (so you can run Pinnacle in your graphical environment)
- [x] Udev backend (so you can run Pinnacle in a tty)
- [x] Tag system
- [ ] Layout system
    - [x] Left master stack, corner, dwindle, spiral layouts
    - [ ] Other three master stack directions, floating, magnifier, maximized, and fullscreen layouts
    - [ ] Resizable layouts
- [x] XWayland support
    - This is currently somewhat buggy. If you find a problem, please submit an issue!
- [x] Layer-shell support
    - [ ] wlr-screencopy support
    - [ ] wlr-output-management support
- [ ] Server-side decorations
- [ ] Animations and blur and all that pizazz
- [ ] Widget system
- [ ] The other stuff Awesome has
- [x] Is very cool :thumbsup:


## Dependencies
> I have not tested these. If Pinnacle doesn't work properly with these packages installed, please submit an issue.

You'll need the following packages, as specified by [Smithay](https://github.com/Smithay/smithay):
`libwayland libxkbcommon libudev libinput libgdm libseat`, as well as `xwayland`.
- Arch:
    ```
    sudo pacman -S wayland wayland-protocols libxkbcommon systemd-libs libinput mesa seatd xwayland
    ```
- Debian:
    ```
    sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgdm-dev libseat-dev xwayland
    ```
- NixOS: Use the provided [`shell.nix`](shell.nix).
- TODO: other distros.

You'll also need Lua 5.4 for configuration. **Older versions will not work.** Check with your package manager to see which version you have.

## Building
Build the project with:
```
cargo build [--release]
```

For NixOS users, there is a provided [`shell.nix`](shell.nix) file that you can use for `nix-shell`.
<sup>flake soon:tm:</sup>

## Running
> :information_source: Before running, read the information in [Configuration](#configuration).

After building, run the executable located in either:
```sh
./target/debug/pinnacle     # without --release
./target/release/pinnacle   # with --release
```

Or, run the project directly with 
```sh
cargo run [--release]
```


Pinnacle will automatically initialize the correct backend for your environment.

However, there is an additional flag you can pass in: `--<backend>`. You most likely do not need to use it.

`backend` can be one of two values:

- `winit`: run Pinnacle as a window in your graphical environment
- `udev`: run Pinnacle in a tty.

If you try to run either in environments where you shouldn't be, you will get a warning requiring you to
pass in the `--force` flag to continue. *You probably shouldn't be doing that.*

> #### :information_source: Make sure `command` in your `metaconfig.toml` is set to the right file.
> If it isn't, the compositor will open, but your config will not apply.
In that case, kill the compositor using the keybind defined in 
`kill_keybind` (default <kbd>Ctrl</kbd><kbd>Alt</kbd><kbd>Shift</kbd> + <kbd>Esc</kbd>) and set `command` properly.

> #### :information_source: Pinnacle will open a socket in the `/tmp` directory.
> If for whatever reason you need the socket to be in a different place, set `socket_dir` in
> your `metaconfig.toml` file to a directory of your choosing.

> #### :warning: Do not run Pinnacle as root.
> This will open the socket with root-only permissions, and future non-root invocations
of Pinnacle will fail when trying to remove the socket until it is removed manually.

## Configuration
Pinnacle is configured in Lua. Rust support is planned.

Pinnacle will search for a `metaconfig.toml` file in the following directories, from top to bottom:
```sh
$PINNACLE_CONFIG_DIR
$XDG_CONFIG_HOME/pinnacle/
~/.config/pinnacle
```

The `metaconfig.toml` file provides information on what config to run, kill and reload keybinds,
and any environment variables you want set. For more details, see the provided 
[`metaconfig.toml`](api/lua/metaconfig.toml) file.

To use the provided Lua config, run the following in the root of the Git project:
```sh
PINNACLE_CONFIG_DIR="./api/lua" cargo run
```

To run without the above environment variable, copy [`metaconfig.toml`](api/lua/metaconfig.toml) and
[`example_config.lua`](api/lua/example_config.lua) to `$XDG_CONFIG_HOME/pinnacle/`
(this will probably be `~/.config/pinnacle`).

> If you rename `example_config.lua` to something like `init.lua`, you will need to change `command` in `metaconfig.toml` to reflect that.

### :information_source: Using the Lua Language Server
It is ***highly*** recommended to use the [Lua language server](https://github.com/LuaLS/lua-language-server)
and set it up to have the [`api/lua`](api/lua) directory as a library.
This will provide documentation, autocomplete, and error checking.

#### For VS Code:
Install the [Lua](https://marketplace.visualstudio.com/items?itemName=sumneko.lua) plugin, then go into
its settings and add the path to the [`api/lua`](api/lua) directory to `Workspace: Library`.

#### For Neovim:
Pass this table into your Lua language server settings:
```lua
Lua = {
    workspace = {
        library = {
            "/path/to/pinnacle/api/lua" -- Your path here
        }
    }
}
```

### API Documentation
You can find online documentation for the Lua API [here](https://ottatop.github.io/pinnacle/main).

Note that there are some missing things like the `Keys` table and `Layout` enum
as well as any function overloads, but these should be autocompleted through the language server.

Documentation for other branches can be reached at `https://ottatop.github.io/pinnacle/<branch name>`.

## Controls
The following controls are currently hardcoded:

- <kbd>Ctrl</kbd> + <kbd>Left click drag</kbd>: Move a window
- <kbd>Ctrl</kbd> + <kbd>Right click drag</kbd>: Resize a window

You can find the rest of the controls in the [`example_config`](api/lua/example_config.lua).

## Feature Requests, Bug Reports, Contributions, and Questions
See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Changelog
See [`CHANGELOG.md`](CHANGELOG.md).
