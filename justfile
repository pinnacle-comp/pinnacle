set shell := ["bash", "-c"]

rootdir := justfile_directory()
xdg_data_dir := `echo "${XDG_DATA_HOME:-$HOME/.local/share}/pinnacle"`

lua_version := "5.4"

local_lua_path := x"$HOME/.luarocks/share/lua/" + lua_version + x"/?.lua;$HOME/.luarocks/share/lua/" + lua_version + "/?.init.lua;"
export LUA_PATH := local_lua_path + env("LUA_PATH", "")
export LUA_CPATH := x"$HOME/.luarocks/lib/lua/" + lua_version + x"/?.so;" + env("LUA_CPATH", "")

list:
    @just --list --unsorted

# Install the protobuf definitions and the Lua library (requires Luarocks)
install: install-protos install-lua-lib install-snowcap

# Install the protobuf definitions (only needed for the Lua API)
install-protos:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    mkdir -p "{{xdg_data_dir}}"
    cp -r "{{rootdir}}/api/protobuf" "${proto_dir}"

# Install the Lua library (requires Luarocks)
install-lua-lib: gen-lua-pb-defs
    #!/usr/bin/env bash
    cd "{{rootdir}}/api/lua"
    luarocks make --local --deps-mode "order" --lua-version "{{lua_version}}" pinnacle-api-dev-1.rockspec

# Remove installed configs and the Lua API (requires Luarocks)
clean: clean-snowcap
    rm -rf "{{xdg_data_dir}}"
    -luarocks remove --local pinnacle-api

# Run `cargo build`
build *args: gen-lua-pb-defs
    cargo build {{args}}

# Generate the protobuf definitions Lua file
gen-lua-pb-defs:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo build --package lua-build
    ./target/debug/lua-build ./api/protobuf > "./api/lua/pinnacle/grpc/defs.lua"

# Run `cargo run`
run *args: gen-lua-pb-defs
    cargo run {{args}}

# Run `cargo test`
test *args: gen-lua-pb-defs
    cargo test --no-default-features --all {{args}}

compile-wlcs:
    #!/usr/bin/env bash
    set -euxo pipefail

    WLCS_SHA=26c5a8cfef265b4ae021adebfec90d758c08792e

    cd "{{rootdir}}"

    if [ -f "./wlcs/wlcs" ] && [ "$(cd wlcs; git rev-parse HEAD)" = "${WLCS_SHA}" ] ; then
        echo "WLCS commit 26c5a8c is already compiled"
    else
        echo "Compiling WLCS"
        git clone https://github.com/canonical/wlcs
        cd wlcs || exit
        # checkout a specific revision
        git reset --hard "${WLCS_SHA}"
        cmake -DWLCS_BUILD_ASAN=False -DWLCS_BUILD_TSAN=False -DWLCS_BUILD_UBSAN=False -DCMAKE_EXPORT_COMPILE_COMMANDS=1 .
        make -j 8
    fi

wlcs *args: compile-wlcs
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo build -p wlcs_pinnacle
    RUST_BACKTRACE=1 ./wlcs/wlcs target/debug/libwlcs_pinnacle.so {{args}}

install-snowcap:
    #!/usr/bin/env bash
    set -euxo pipefail
    cd "{{rootdir}}/snowcap"
    just install

clean-snowcap:
    #!/usr/bin/env bash
    set -euxo pipefail
    cd "{{rootdir}}/snowcap"
    just clean
