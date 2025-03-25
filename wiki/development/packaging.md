---
outline: [2, 5]
---

# Packaging

## Building

Remember to build/fetch dependencies with `--locked`, as this uses the same versions
of dependencies as in development.

### Features

If you want to package Pinnacle without building in the Snowcap widget system, build with
`--no-default-features`. Other than that, you won't need to enable any other features as
the rest are for testing purposes.

## Installation

### Session files

It is recommended to package Pinnacle as a session so that display managers pick it up and
to allow XDG desktop portals to function.

To do this, place the following files in the respective destination directory:

| File                                 | Destination                      |
| ------------------------------------ | -------------------------------- |
| `target/release/pinnacle`            | `/usr/bin/`                      |
| `resources/pinnacle-session`         | `/usr/bin/`                      |
| `resources/pinnacle.desktop`         | `/usr/share/wayland-sessions/`   |
| `resources/pinnacle-portals.conf`    | `/usr/share/xdg-desktop-portal/` |
| `resources/pinnacle.service`         | `/usr/lib/systemd/user/`         |
| `resources/pinnacle-shutdown.target` | `/usr/lib/systemd/user/`         |

### The Lua API

It is also recommended to package the Lua API together with the compositor for
a streamlined experience. This enables the use of `pinnacle client` and
easy scripting for external applications.

You can use [LuaRocks](https://luarocks.org/) to package and install the Lua API.
Alternatively, because the Lua API consists of only Lua files, you can manually copy them
for installation.

> [!NOTE]
> The Lua API supports Lua 5.2 and up. Replace `<lua-version>` with the version
> you are packaging for.

#### Dependencies

The Lua API requires the following Lua dependencies to be installed
in `/usr/share/lua/<lua-version>` (or wherever Lua is set up to search for requires):

- [`lua-http`](https://github.com/daurnimator/lua-http)
- [`lua-protobuf`](https://github.com/starwing/lua-protobuf)
- [`cqueues`](https://github.com/wahern/cqueues)
- [`luaposix`](https://github.com/luaposix/luaposix)
- [`lua-compat53`](https://github.com/lunarmodules/lua-compat-5.3)
    (optional, to enable support for Lua 5.2)

If your distribution's package manager has these packages, add them as dependencies.

If not and there is no way to create them and you have no community package repo,
you may be able to build these dependencies and copy them under the `api/lua/pinnacle` directory.
Note that you will have to patch the API to allow `require` to find the dependencies.

#### Installing with LuaRocks

> [!NOTE]
> The following is based on the Arch PKGBUILD system. I don't know if your distribution
> does things a little differently.

To use LuaRocks to install the Lua API, first pack the API into a rock in the build step.

```sh
cd api/lua

luarocks --lua-version <lua-version> make --pack-binary-rock \
    --deps-mode none --no-manifest <rockspec_for_the_packaged_version>
```

Then, you can install the rock in the package step.

```sh
cd api/lua

luarocks --lua-version <lua-version> --tree "$pkgdir/usr/" \
    install --deps-mode none --no-manifest <packaged-rock>
```

#### Installing manually

Copy the following files and directories to `/usr/share/lua/<lua-version>/`
(or wherever your distribution's Lua install looks for modules):

- `api/lua/pinnacle.lua`
- `api/lua/pinnacle/`

> [!IMPORTANT]
> There is a symlink in the Lua API to include the Snowcap API. Make sure that
> copying follows symlinks so that these files are actually installed.

#### Protobuf files

The Lua API needs to load protobuf definitions at runtime.

Copy the following protobuf definition directories to their destinations:

| Directory               | Destination                    |
| ----------------------- | ------------------------------ |
| `api/protobuf/`         | `/usr/share/pinnacle/`         |
| `snowcap/api/protobuf/` | `/usr/share/pinnacle/snowcap/` |

If your distribution does not use these directories, copy the protobuf files to
someplace in `XDG_DATA_DIRS`.
