set shell := ["bash", "-c"]

rootdir := justfile_directory()
xdg_data_dir := `echo "${XDG_DATA_HOME:-$HOME/.local/share}/snowcap"`
root_xdg_data_dir := "/usr/share/snowcap"

lua_version := "5.4"

list:
    @just --list --unsorted

install: install-protos install-lua-lib

install-protos:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    mkdir -p "{{xdg_data_dir}}"
    cp -r "{{rootdir}}/api/protobuf" "${proto_dir}"

install-lua-lib: gen-lua-pb-defs
    #!/usr/bin/env bash
    set -euxo pipefail
    cd "{{rootdir}}/api/lua"
    luarocks build --local https://raw.githubusercontent.com/pinnacle-comp/lua-grpc-client/main/lua-grpc-client-dev-1.rockspec
    luarocks build --local --lua-version "{{lua_version}}"

clean:
    rm -rf "{{xdg_data_dir}}"
    -luarocks remove --local snowcap-api
    -luarocks remove --local lua-grpc-client

# Generate the protobuf definitions Lua file
gen-lua-pb-defs:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo build --package lua-build
    ./target/debug/lua-build > "./api/lua/snowcap/grpc/defs.lua"
