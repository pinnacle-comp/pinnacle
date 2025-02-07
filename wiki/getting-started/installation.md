# Installation

## Distro-specific

It is recommended that you install Pinnacle using your distro's package manager.

TODO: lmao we don't have any packages yet

## From Source

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

> [!NOTE]
> You must run `eval $(luarocks path --lua-version <your-lua-version>)` so that your config can find the API
> library files. It is recommended to place this command in your shell's startup script.

### Building

Clone the repository.
```sh
git clone https://github.com/pinnacle-comp/pinnacle
```

To build Pinnacle, run `just`, passing `install` to install the Lua API with Luarocks.
This passes through arguments to Cargo.
```sh
just install build [--release]
```

### Running

Run Pinnacle with `just`.
```sh
just run [--release]
```

You can press `Super`+`S` to bring up the bind overlay.
