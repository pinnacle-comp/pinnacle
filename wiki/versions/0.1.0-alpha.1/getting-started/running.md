# Running

Run Pinnacle with `just`.
```sh
just run [--release]
```

You can run Pinnacle within another desktop environment, compositor, or window manager.
In this case, it will open as a nested window. This is useful if you want to quickly
try Pinnacle out.

On the first startup, assuming you haven't generated a config yet, Pinnacle will spin up
the builtin Rust config.

### Key and mousebinds

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
