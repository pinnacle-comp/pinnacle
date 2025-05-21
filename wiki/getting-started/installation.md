# Installation

## Distro-specific

It is recommended that you install Pinnacle using your distro's package manager.

::: tabs

== Arch (AUR)
```sh
yay -S pinnacle-comp

# Or latest commit
yay -S pinnacle-comp-git
```

> [!IMPORTANT]
> The `-git` package is currently broken. Use the non `-git` version or build from source.

:::

## From source

Alternatively, you can build and install Pinnacle from source.

### Dependencies

To build the project, you will need Rust 1.82 or newer.

First, you will need the following dependencies:
- [`just`](https://github.com/casey/just)
- `libwayland`
- `libxkbcommon`
- `libudev`
- `libinput`
- `libgbm`
- `libseat`
- `libEGL`
- `libsystemd`
- `libdisplay-info` for monitor display information
- `xwayland` for Xwayland support
- [`protoc`](https://grpc.io/docs/protoc-installation/) for the API

To configure Pinnacle using Lua, you will also need:
- [`lua`](https://www.lua.org/) 5.2 or newer
- [`luarocks`](https://luarocks.org/) for API installation

### Building

Clone the repository.
```sh
git clone https://github.com/pinnacle-comp/pinnacle
```

To build Pinnacle, run `just build`. This passes through arguments to Cargo.
```sh
just build [Cargo arguments...]
```

To use the Lua API, you will also need to run `just install`. This will install the protobuf files
and Lua API locally.

> [!TIP]
> You can run multiple `just` recipes in one command, e.g.
> ```sh
> just install build
> ```
