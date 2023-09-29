# Changelog

## [a109c70](https://github.com/pinnacle-comp/pinnacle/commit/a109c704ec371640829e375bf787db13540330d1) [(#96)](https://github.com/pinnacle-comp/pinnacle/pull/96) (28 Sep 2023)
There are now API options for xkeyboard and libinput settings.

### Changes
- Add xkbconfig to API
- Add libinput config options to API
- Update keyboard LEDs

## [5e49d77](https://github.com/pinnacle-comp/pinnacle/commit/5e49d77ef8cdbd063a8a8bd3b66b3474cb150d78) [(#94)](https://github.com/pinnacle-comp/pinnacle/pull/94) (21 Sep 2023)
This update brings breaking changes to the `metaconfig.toml` file.

If you have an old one, you will need to:
- Change `command` to be an array of arguments instead of one string, and
- Change all references to `$PINNACLE_DIR/api` to `$PINNACLE_LIB_DIR`.

Additionally, the default config will now automatically load if your's crashes or can't be found.

### Changes
- Change `command` in `metaconfig.toml` to take in an array of strings instead of a single string
    - Fixes a problem due to me splitting the command by spaces
- Change `$PINNACLE_DIR/api` to be `$PINNACLE_LIB_DIR` instead
- Load default config if current one crashes or can't be found
- Pinnacle now copies the Lua library to `$XDG_DATA_HOME/pinnacle` (or `~/.local/share/pinnacle`)

## [8499a29](https://github.com/pinnacle-comp/pinnacle/commit/8499a291e2225f00b2d745381915f7cffc570d37) [(#78)](https://github.com/pinnacle-comp/pinnacle/pull/78)
This update brings mousebinds to the config API. You can now do things on button press and release.

### Changes
- Add mousebinds to API
- Add env setting to API

### Bugfixes
- Correct scroll direction on Winit

## [01b6e25](https://github.com/Ottatop/pinnacle/commit/01b6e258ff72a5517e2c653f058f5241fa953162) [(#65)](https://github.com/Ottatop/pinnacle/pull/65)
This update adds an initial window rules implementation! There are only a few conditions and rules to start,
but this is expected to grow over time as I add more.

### Changes
- Add window rules

## [43949e3](https://github.com/Ottatop/pinnacle/commit/43949e386dd6ddd2092699ca6ec2109dd65f3d5a) [(#56)](https://github.com/Ottatop/pinnacle/pull/56)
This update brings breaking changes to configuration.

You'll now need a `metaconfig.toml` file to tell Pinnacle to run a Lua config.
You can copy the provided [`metaconfig.toml`](api/lua/metaconfig.toml) file to `~/.config/pinnacle`
or wherever you have your config files.

To continue using the provided Lua config, you now need to run
```sh
PINNACLE_CONFIG_DIR="./api/lua" cargo run
```
instead of using `PINNACLE_CONFIG`.

This update also brings config reloading! You can now update your config and reload on the fly
without having to restart the compositor. If your config crashes, you can also reload to restart it.

### Changes
- Add `metaconfig.toml` file
- Add config reloading

## [3cc462d](https://github.com/Ottatop/pinnacle/commit/3cc462de2c0b34ec593e87bd5c9377dba19a0cc9) [(#53)](https://github.com/Ottatop/pinnacle/pull/53)

### Changes
- Add fullscreen and maximized window support

### Known bugs
- There is slight flickering then changing a window to and from floating
- Xwayland fullscreen requests are currently ~~really buggy~~ basically unusable
    - Fullscreen window sizing won't update unless the tag is changed
    - Some windows may disappear when toggling off fullscreen

## [4261b6e](https://github.com/Ottatop/pinnacle/commit/4261b6e60fc17219f76bf1dc835e0abc9baceaeb) [(#45)](https://github.com/Ottatop/pinnacle/pull/45)

### Changes
- Add wlr-layer-shell support

## [ba7b259](https://github.com/Ottatop/pinnacle/commit/ba7b2597f17c3af375f19c1eb8a29abe74d2bd61) [(#34)](https://github.com/Ottatop/pinnacle/pull/34)

### Changes
- Add XWayland support
