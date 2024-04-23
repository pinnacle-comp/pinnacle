set shell := ["bash", "-c"]

rootdir := justfile_directory()
xdg_data_dir := `echo "${XDG_DATA_HOME:-~/.local/share}/pinnacle"`
root_xdg_data_dir := "/usr/share/pinnacle"
root_xdg_config_dir := "/etc/xdg/pinnacle"

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
    mkdir -p "${default_config_dir}"
    cp -r "{{rootdir}}/api/lua/examples/default" "${default_lua_dir}"
    cp -LR "{{rootdir}}/api/rust/examples/default_config/for_copying" "${default_rust_dir}"

# Install the protobuf definitions (only needed for the Lua API)
install-protos:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    mkdir -p "${proto_dir}"
    cp -r "{{rootdir}}/api/protocol" "${proto_dir}"

# Install the Lua library (requires Luarocks)
install-lua-lib:
    #!/usr/bin/env bash
    cd "{{rootdir}}/api/lua"
    luarocks make --local

# Remove installed configs and the Lua API (requires Luarocks)
clean:
    rm -rf "{{xdg_data_dir}}"
    -luarocks remove --local pinnacle-api

# [root] Remove installed configs and the Lua API (requires Luarocks)
clean-root:
    rm -rf "{{root_xdg_data_dir}}"
    rm -rf "{{root_xdg_config_dir}}"
    -luarocks remove pinnacle-api

# [root] Install the configs, protobuf definitions, and the Lua library (requires Luarocks)
install-root: install-configs-root install-protos-root install-lua-lib-root

# [root] Install the default Lua and Rust configs
install-configs-root:
    #!/usr/bin/env bash
    set -euxo pipefail
    default_config_dir="{{root_xdg_config_dir}}/default_config"
    default_lua_dir="${default_config_dir}/lua"
    default_rust_dir="${default_config_dir}/rust"
    rm -rf "${default_config_dir}"
    mkdir -p "${default_config_dir}"
    cp -r "{{rootdir}}/api/lua/examples/default" "${default_lua_dir}"
    cp -LR "{{rootdir}}/api/rust/examples/default_config/for_copying" "${default_rust_dir}"

# [root] Install the protobuf definitions (only needed for the Lua API)
install-protos-root:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{root_xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    mkdir -p "${proto_dir}"
    cp -r "{{rootdir}}/api/protocol" "${proto_dir}"

# [root] Install the Lua library (requires Luarocks)
install-lua-lib-root:
    #!/usr/bin/env bash
    cd "{{rootdir}}/api/lua"
    luarocks make

# Run `cargo build`
build *args:
    cargo build {{args}}

# Run `cargo run`
run *args:
    cargo run {{args}}

# Run `cargo test`
test *args:
    cargo test {{args}}