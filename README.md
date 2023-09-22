![Pinnacle banner](/assets/pinnacle_banner_dark.png)

https://github.com/Ottatop/pinnacle/assets/120758733/c175ba80-9796-4759-92c3-1d7a6639b0c9

# Info
### What is Pinnacle?
Pinnacle is a Wayland compositor built in Rust using [Smithay](https://github.com/Smithay/smithay).
It's my attempt at creating something like [AwesomeWM](https://github.com/awesomeWM/awesome)
for Wayland.

It sports high configurability through a (soon to be) extensive Lua API, with plans for a Rust API in the future.

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


# Dependencies
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

You'll also need Lua 5.4 for configuration. **Older versions will not work.**
Check with your package manager to see which version you have.

# Building
Build the project with:
```
cargo build [--release]
```

For NixOS users, there is a provided [`shell.nix`](shell.nix) file that you can use for `nix-shell`.
<sup>flake soon:tm:</sup>

> [!NOTE]
> On build, [`install_libs.sh`](install_libs.sh) will run to copy the Lua API library to
> `$XDG_DATA_HOME/pinnacle` (or `~/.local/share/pinnacle`).

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
Pinnacle is configured in Lua. Rust support is planned.

Pinnacle will search for a `metaconfig.toml` file in the following directories, from top to bottom:
```sh
$PINNACLE_CONFIG_DIR
$XDG_CONFIG_HOME/pinnacle
~/.config/pinnacle # Only if $XDG_CONFIG_HOME is not defined
```

The `metaconfig.toml` file provides information on what config to run, kill and reload keybinds,
and any environment variables you want set. For more details, see the provided 
[`metaconfig.toml`](api/lua/metaconfig.toml) file.

If no `metaconfig.toml` file is found, the default config will be loaded.


For custom configuration, you can copy [`metaconfig.toml`](api/lua/metaconfig.toml) and
[`example_config.lua`](api/lua/example_config.lua) to `$XDG_CONFIG_HOME/pinnacle`
(this will probably be `~/.config/pinnacle`).

> Make sure `command` in your `metaconfig.toml` is set to the right file.
> If it isn't, the compositor will load the default config instead.
> 
> If you rename `example_config.lua` to something like `init.lua`,
> you will need to change `command` to reflect that.

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
You can find online documentation for the Lua API [here](https://pinnacle-comp.github.io/pinnacle/main).

This documentation is auto-generated from the provided LuaLS annotation through
[ldoc_gen](https://github.com/Ottatop/ldoc_gen), so there may be some errors as I work the kinks out.

Note that there are some missing things like the `Keys` table and `Layout` enum
as well as any function overloads, but these should be autocompleted through the language server.

Documentation for other branches can be reached at `https://pinnacle-comp.github.io/pinnacle/<branch name>`.

# Controls
The following are the default controls in the [`example_config`](api/lua/example_config.lua).
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
