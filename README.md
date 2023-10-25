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
- Tag system
- Left master stack, corner, dwindle, and spiral layouts from Awesome
- XWayland support
- Layer-shell support
- Configurable in Lua or Rust
- Is very cool :thumbsup:

### Roadmap
- TODO

# Dependencies
You will need Rust installed to compile this project and use the Rust API for configuration.

You'll also need the following packages, as specified by [Smithay](https://github.com/Smithay/smithay):
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

If you're configuring Pinnacle using Lua, you'll additionally need Lua 5.4 for configuration.
**Older versions will not work.**
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
Pinnacle is configured in your choice of Lua or Rust.

## Out-of-the-box configurations
If you just want to test Pinnacle out without copying stuff to your config directory,
run either of the following in the crate root:
```sh
# For a Lua configuration
PINNACLE_CONFIG_DIR="./api/lua" cargo run

# For a Rust configuration
PINNACLE_CONFIG_DIR="./api/rust" cargo run
```

## Custom configuration

> [!IMPORTANT]
> Pinnacle is under heavy development, and there *will* be major breaking changes to these APIs
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
[`metaconfig.toml`](api/lua/metaconfig.toml) file.

If no `metaconfig.toml` file is found, the default Lua config will be loaded.

### Lua
For custom configuration in Lua, you can copy [`metaconfig.toml`](api/lua/metaconfig.toml) and 
[`example_config.lua`](api/lua/example_config.lua) to `$XDG_CONFIG_HOME/pinnacle`
(this will probably be `~/.config/pinnacle`).

> If you rename `example_config.lua`, make sure `command` in your `metaconfig.toml` is updated to reflect that.
> If it isn't, the compositor will load the default config instead.

#### :information_source: Using the Lua Language Server
It is ***highly*** recommended to setup your [Lua language server](https://github.com/LuaLS/lua-language-server)
installation to use the [`api/lua`](api/lua) directory as a library.
This will provide documentation, autocomplete, and error checking.

The Lua library should have been copied to `$XDG_DATA_HOME/pinnacle` (or `~/.local/share/pinnacle`).

##### For VS Code:
Install the [Lua](https://marketplace.visualstudio.com/items?itemName=sumneko.lua) plugin, then go into
its settings and add the path above to the [`api/lua`](api/lua) directory to `Workspace: Library`.

##### For Neovim:
Pass this table into your Lua language server settings:
```lua
Lua = {
    workspace = {
        library = {
            "$XDG_DATA_HOME/pinnacle/lua", -- Replace $XDG_DATA_HOME with the full path
            -- OR
            "$HOME/.local/share/pinnacle/lua", -- Replace $HOME with the full path
        }
    }
}
```

### Rust
If you want to use Rust to configure Pinnacle, follow these steps:
1. In `~/.config/pinnacle`, run `cargo init`.
2. In the `Cargo.toml` file, add the following under `[dependencies]`:
```toml
# rev is HIGHLY recommended to prevent breaking changes
pinnacle_api = { git = "http://github.com/pinnacle-comp/pinnacle", rev = "..." }
```
3. Create the file `metaconfig.toml` at the root. Add the following to the file:
```toml
command = ["cargo", "run"]
reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }
```
4. Copy the contents from [`example_config.rs`](api/rust/examples/example_config.rs) to `src/main.rs`.
5. Run Pinnacle! (You may want to run `cargo build` beforehand so you don't have to wait for your config to compile.)


### API Documentation
<b>Lua: https://pinnacle-comp.github.io/pinnacle/main/lua.<br>
Rust: https://pinnacle-comp.github.io/pinnacle/main/rust.</b>

> Documentation for other branches can be reached by replacing `main` with the branch you want.

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
