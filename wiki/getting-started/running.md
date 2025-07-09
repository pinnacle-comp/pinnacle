# Running

### From a package

Pinnacle should show up in your display manager. Simply select it and log in.

### From source

Run Pinnacle with `just`.
```sh
just run [Cargo arguments...]
```

> [!TIP]
> If Pinnacle was compiled with Snowcap integration (on by default), you will want Vulkan set up properly,
> otherwise widgets will fallback to software rendering, which is slow.
>
> For those using Nix outside of NixOS, you will want to run the built binary
> with [nixGL](https://github.com/nix-community/nixGL) using *both* GL and Vulkan wrappers, nested inside one another:
> ```
> nix run --impure github:nix-community/nixGL -- nix run --impure github:nix-community/nixGL#nixVulkanIntel -- ./target/debug/pinnacle
> ```

### Running as a session

Running Pinnacle as a session allows D-Bus and XDG desktop portals to work for things like
PipeWire screen capture and Flatpak apps.

Starting Pinnacle from a display manager will automatically run it as a session.

To run Pinnacle as a session outside of a display manager:

- If you are on a systemd distribution, run `pinnacle-session`. This will start Pinnacle as a systemd service
  in addition to running `pinnacle --session` as mentioned below.
- Otherwise, run `pinnacle --session`. This will import the necessary environment variables into D-Bus
  and systemd, it if is running.

### Within another graphical environment

You can run Pinnacle within another desktop environment, compositor, or window manager.
In this case, it will open as a nested window. This is useful if you want to quickly
try Pinnacle out.

## Key and mousebinds

You can press `Super`+`S` to bring up the bind overlay. Below are the default binds.

`Mod` is `Super` when running in a tty and `Alt` when running as a nested window.

| Binding                           | Action                                                    |
|-----------------------------------|-----------------------------------------------------------|
| `Mod` + `s`                       | Show the keybind overlay                                  |
| `Mod` + `Mouse left drag`         | Move window                                               |
| `Mod` + `Mouse right drag`        | Resize window                                             |
| `Mod` `Shift` + `q`               | Quit Pinnacle                                             |
| `Mod` `Ctrl` + `r`                | Reload the config                                         |
| `Mod` `Shift` + `c`               | Close window                                              |
| `Mod` + `Return`                  | Spawn [Alacritty](https://github.com/alacritty/alacritty) |
| `Mod` `Ctrl` + `Space`            | Toggle floating                                           |
| `Mod` + `f`                       | Toggle fullscreen                                         |
| `Mod` + `m`                       | Toggle maximized                                          |
| `Mod` + `Space`                   | Cycle to the next layout                                  |
| `Mod` `Shift` + `Space`           | Cycle to the previous layout                              |
| `Mod` + `1` to `9`                | Switch to tag `1` to `9`                                  |
| `Mod` `Ctrl` + `1` to `9`         | Toggle tag `1` to `9`                                     |
| `Mod` `Shift` + `1` to `9`        | Move a window to tag `1` to `9`                           |
| `Mod` `Ctrl` `Shift` + `1` to `9` | Toggle tag `1` to `9` on a window                         |

## Other run options

- `--config-dir` / `-c`: Uses the config at the specified directory
- `--no-xwayland`: Prevents Xwayland from being spawned
- `--no-config`: Prevents your config from spawning
