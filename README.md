# <div align="center">Pinnacle</div>
<div align="center">
    <picture>
        <source media="(prefers-color-scheme: dark)" srcset="/assets/cool_logo_dark_theme.png">
        <source media="(prefers-color-scheme: light)" srcset="/assets/cool_logo_light_theme.png">
        <img alt="Cool logo" src="/assets/cool_logo_dark_theme.png">
    </picture>
</div>

<div align="center">
    A very, VERY WIP Smithay-based wayland compositor
</div>

## API Documentation
~~There is now *finally* [some form of documentation](https://github.com/Ottatop/pinnacle/wiki/API-Documentation) so you don't have to dig around in the code. It isn't great and automating it seems like a pain, but hey it's something! This may become out of date real quick though, and I'm probably going to need to move to LDoc in the future.~~

Alrighty there's now a preliminary [doc website](https://ottatop.github.io/pinnacle/) generated with LDoc! There are some missing things like the `Keys` and `Layout` tables as well as any function overloads, but you can now click through @see annotations!

## Features
- [x] Winit backend
- [x] Udev backend
    - This is currently just a copy of Anvil's udev backend.
- [x] Basic tags
- [ ] Layout system
    - [x] Left master stack, corner, dwindle, spiral layouts
    - [ ] Other three master stack directions, floating, magnifier, maximized, and fullscreen layouts
    - [ ] Resizable layouts
- [ ] XWayland support
- [ ] Layer-shell support
- [ ] Server-side decorations
- [ ] Animations and blur and all that pizazz
- [ ] Widget system
- [ ] The other stuff Awesome has
- [x] Is very cool :thumbsup:

## Info
### Why Pinnacle?
Well, I currently use [Awesome](https://github.com/awesomeWM/awesome). And I really like it! Unfortunately, Awesome doesn't exist for Wayland ([anymore](http://way-cooler.org/blog/2020/01/09/way-cooler-post-mortem.html)). There doesn't seem to be any Wayland compositor out there that has all of the following:
- Tags for window management
- Configurable in Lua (or any other programming language for that matter)
- Has a bunch of batteries included (widget system, systray, etc)

So, this is my attempt at making an Awesome-esque Wayland compositor.

## Dependencies
You'll need the following packages, as specified by [Smithay](https://github.com/Smithay/smithay):
`libwayland libxkbcommon libudev libinput libgdm libseat`
- Arch:
    ```
    sudo pacman -S wayland libxkbcommon systemd-libs libinput libgdm seatd
    ```
- Debian:
    ```
    sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgdm-dev libseat-dev
    ```
- TODO: other distros.

You'll also need Lua 5.4 for configuration.

## Building
Build the project with:
```
cargo build [--release]
```

## Running
After building, run the executable located in either:
```
./target/debug/pinnacle --<backend>     // without --release
./target/release/pinnacle --<backend>   // with --release
```

Or, run the project directly with 
```
cargo run [--release] -- --<backend>
```

When running in debug mode, the compositor will drastically slow down if there are too many windows on screen. If you don't want this to happen, use release mode.

`backend` can be one of two values:

- `winit`: run Pinnacle as a window in your graphical environment
- `udev`: run Pinnacle in a tty. NOTE: I tried running udev in Awesome and some things broke so uh, don't do that

## Configuration
Please note: this is VERY WIP and has few options.

Pinnacle supports configuration through Lua (and hopefully more languages if it's not too unwieldy :crab:).

Run Pinnacle with the `PINNACLE_CONFIG` environment variable set to the path of your config file. If not specified, Pinnacle will look for the following: 
```
$XDG_CONFIG_HOME/pinnacle/init.lua
~/.config/pinnacle/init.lua         // if XDG_CONFIG_HOME isn't set
```
The following will use the example config file in [`api/lua`](api/lua):
```
PINNACLE_CONFIG="./api/lua/example_config.lua" cargo run -- --<backend>
```

### Autocomplete and that cool stuff
It is *highly* recommended to use the [Lua language server](https://github.com/LuaLS/lua-language-server) and set it up to have the [`api/lua`](api/lua) directory as a library, as I'll be using its doc comments to provide autocomplete and error checking.

#### For VS Code:
Install the [Lua](https://marketplace.visualstudio.com/items?itemName=sumneko.lua) plugin, then go into its settings and add the absolute(?) path to the [`api/lua`](api/lua) directory to Workspace: Library.

#### For Neovim:
Pass this table into your Lua language server settings:
```lua
Lua = {
    workspace = {
        library = {
            "/path/to/pinnacle/api/lua"
        }
    }
}
```

Doc website soon:tm:

## Controls
The following controls are currently hardcoded:

- `Ctrl + Left Mouse`: Move a window
- `Ctrl + Right Mouse`: Resize a window
- `Ctrl + Alt + Shift + Esc`: Kill Pinnacle. This is for when the compositor inevitably locks up because I did a dumb thing :thumbsup:

You can find the rest of the controls in the [`example_config`](api/lua/example_config.lua).
