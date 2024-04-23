set shell := ["bash", "-c"]

rootdir := justfile_directory()
xdg_data_dir := `echo "${XDG_DATA_HOME:-~/.local/share}/pinnacle"`

list:
    @just --list --unsorted

# Install the configs, protobuf definitions, and the Lua library (requires Luarocks)
install: install-configs install-protos install-lua-lib

# Install the default Lua and Rust configs
install-configs:
    #!/usr/bin/env bash
    set -euxo pipefail
    default_config_dir="{{xdg_data_dir}}/default_config"
    default_lua_dir="${default_config_dir}/lua"
    default_rust_dir="${default_config_dir}/rust"
    rm -rf "${default_config_dir}"
    mkdir "${default_config_dir}"
    cp -r "{{rootdir}}/api/lua/examples/default" "${default_lua_dir}"
    cp -LR "{{rootdir}}/api/rust/examples/default_config/for_copying" "${default_rust_dir}"

# Install the protobuf definitions (only needed for the Lua API)
install-protos:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    cp -r "{{rootdir}}/api/protocol" "${proto_dir}"

# Install the Lua library (requires Luarocks)
install-lua-lib:
    #!/usr/bin/env bash
    cd "{{rootdir}}/api/lua"
    luarocks make --local

build *args: install
    cargo build {{args}}

run *args: install
    cargo run {{args}}

test *args: install
    cargo test {{args}}
