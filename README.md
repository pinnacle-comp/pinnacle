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

## News
- [Wlr-layer-shell support](https://github.com/Ottatop/pinnacle/pull/45) is now here!
Now you can use stuff like [swaybg](https://github.com/swaywm/swaybg) so you don't have
to look at an ugly gray background and [eww](https://github.com/elkowar/eww)
for widgets (until I implement a widget system, that is). As always, if you find any
issues, submit a bug report!

<details>

<summary>Older stuff</summary>

- We now have XWayland support as of [#34](https://github.com/Ottatop/pinnacle/pull/34)!
It's currently not that polished right now because I got bored of working on it and I want
to work on other aspects of Pinnacle, but it should be at least *usable*.

</details>

## Features
- [x] Winit backend
- [x] Udev backend
    - This is currently just a copy of Anvil's udev backend.
- [x] Basic tags
- [ ] Layout system
    - [x] Left master stack, corner, dwindle, spiral layouts
    - [ ] Other three master stack directions, floating, magnifier, maximized, and fullscreen layouts
    - [ ] Resizable layouts
- [x] XWayland support
    - This is currently somewhat buggy. If you find a problem that's not already listed in GitHub issues, feel free to submit it!
- [x] Layer-shell support
    - [ ] wlr-screencopy support
    - [ ] wlr-output-management support
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
    sudo pacman -S wayland wayland-protocols libxkbcommon systemd-libs libinput mesa seatd
    ```
- Debian:
    ```
    sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgdm-dev libseat-dev
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
It *should* work, but if it doesn't, please raise an issue. <sup>flake soon:tm:</sup>

## Running
After building, run the executable located in either:
```sh
./target/debug/pinnacle --<backend>     # without --release
./target/release/pinnacle --<backend>   # with --release
```

Or, run the project directly with 
```sh
cargo run [--release] -- --<backend>
```

`backend` can be one of two values:

- `winit`: run Pinnacle as a window in your graphical environment
- `udev`: run Pinnacle in a tty. NOTE: I tried running udev in Awesome and some things broke so uh, don't do that

> :information_source: When running in debug mode, the compositor will drastically slow down
> if there are too many windows on screen. If you don't want this to happen, use release mode.

> #### :exclamation: IMPORTANT: Read the following before you launch the `udev` backend:
> If you successfully enter the `udev` backend but none of the controls work, this means either Pinnacle
failed to find your config, or the config process crashed.
> 
> I have not yet implemented VT switching, so to enable you to exit the compositor if this happens,
> ```
> Ctrl + Alt + Shift + Escape
> ```
> has been hardcoded in to kill the compositor.

> #### :information_source: Pinnacle will open a socket in the `/tmp` directory.
> If for whatever reason you need the socket to be in a different place, run Pinnacle with
> the `SOCKET_DIR` environment variable:
> ```sh
> SOCKET_DIR=/path/to/new/dir/ cargo run -- --<backend>
> ```

> #### :warning: Don't run Pinnacle as root.
> This will open the socket with root-only permissions, and future non-root invocations
of Pinnacle will fail when trying to remove the socket until it is removed manually.

## Configuration
Please note: this is WIP and has few options.

Pinnacle supports configuration through Lua (and hopefully more languages if it's not too unwieldy :crab:).

Run Pinnacle with the `PINNACLE_CONFIG` environment variable set to the path of your config file. If not specified, Pinnacle will look for the following: 
```sh
$XDG_CONFIG_HOME/pinnacle/init.lua
~/.config/pinnacle/init.lua         # if XDG_CONFIG_HOME isn't set
```
The following will use the example config file in [`api/lua`](api/lua):
```sh
PINNACLE_CONFIG="./api/lua/example_config.lua" cargo run -- --<backend>
```

> ##### :information_source: The config is an external process.
> If it crashes for whatever reason, all of your keybinds will stop working.
> Again, you can exit the compositor with `Ctrl + Alt + Shift + Escape`.
>
> Config reloading soon:tm:

### API Documentation
There is a preliminary [doc website](https://ottatop.github.io/pinnacle/main) generated with LDoc.
Note that there are some missing things like the `Keys` table and `Layout` enum
as well as any function overloads, but these should be autocompleted through the language server.

Documentation for other branches can be reached at `https://ottatop.github.io/pinnacle/<branch name>`.

### Autocomplete and that cool stuff
It is *highly* recommended to use the [Lua language server](https://github.com/LuaLS/lua-language-server)
and set it up to have the [`api/lua`](api/lua) directory as a library, as I'll be using
its doc comments to provide documentation, autocomplete, and error checking.

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

## Controls
The following controls are currently hardcoded:

- `Ctrl + Left Mouse`: Move a window
- `Ctrl + Right Mouse`: Resize a window
- `Ctrl + Alt + Shift + Esc`: Kill Pinnacle. This is for when the compositor inevitably
locks up because I did a dumb thing :thumbsup:

You can find the rest of the controls in the [`example_config`](api/lua/example_config.lua).

## Feature Requests, Bug Reports, Contributions, and Questions
See [CONTRIBUTING.md](CONTRIBUTING.md).
