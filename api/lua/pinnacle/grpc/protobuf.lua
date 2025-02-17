-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

require("compat53")

local stat = require("posix.sys.stat").stat
local pb = require("pb")

local protobuf = {}

function protobuf.build_protos()
    require("pinnacle.log"):debug("Building protos")

    local version = "v1"
    local proto_file_paths = {
        "pinnacle/tag/" .. version .. "/tag.proto",
        "pinnacle/input/" .. version .. "/input.proto",
        "pinnacle/" .. version .. "/pinnacle.proto",
        "pinnacle/output/" .. version .. "/output.proto",
        "pinnacle/process/" .. version .. "/process.proto",
        "pinnacle/window/" .. version .. "/window.proto",
        "pinnacle/signal/" .. version .. "/signal.proto",
        "pinnacle/layout/" .. version .. "/layout.proto",
        "pinnacle/render/" .. version .. "/render.proto",
        "pinnacle/util/" .. version .. "/util.proto",
        "google/protobuf/empty.proto",
    }

    local xdg_data_home = os.getenv("XDG_DATA_HOME")
    local xdg_data_dirs = os.getenv("XDG_DATA_DIRS")

    print(xdg_data_home)
    print(xdg_data_dirs)

    ---@type string[]
    local search_dirs = {}

    if xdg_data_home then
        table.insert(search_dirs, xdg_data_home)
    end

    if xdg_data_dirs then
        for data_dir in xdg_data_dirs:gmatch("[^:]+") do
            table.insert(search_dirs, data_dir)
        end
    end

    local proto_dir = nil

    for _, dir in ipairs(search_dirs) do
        if stat(dir .. "/pinnacle/protobuf") then
            proto_dir = dir .. "/pinnacle/protobuf"
        end
    end

    print(proto_dir)

    assert(proto_dir, "could not find protobuf definitions directory")

    local cmd = "protoc --descriptor_set_out=/tmp/pinnacle.pb --proto_path=" .. proto_dir .. " "

    for _, file_path in ipairs(proto_file_paths) do
        cmd = cmd .. file_path .. " "
    end

    local proc = assert(io.popen(cmd), "protoc is not installed")
    local _ = proc:read("a")
    proc:close()

    local pinnacle_pb = assert(io.open("/tmp/pinnacle.pb", "r"), "no pb file generated")
    local pinnacle_pb_data = pinnacle_pb:read("a")
    pinnacle_pb:close()

    assert(pb.load(pinnacle_pb_data), "failed to load .pb file")

    pb.option("enum_as_value")
end

---Encode the given `data` as the protobuf `type`.
---@param type string The absolute protobuf type
---@param data table The table of data, conforming to its protobuf definition
---@return string buffer The encoded buffer
function protobuf.encode(type, data)
    local success, obj = pcall(pb.encode, type, data)
    if not success then
        print("failed to encode:", obj, "type:", type)
        os.exit(1)
    end

    local encoded_protobuf = obj

    ---@diagnostic disable-next-line: deprecated
    local packed_prefix = string.pack("I1", 0)
    ---@diagnostic disable-next-line: deprecated
    local payload_len = string.pack(">I4", encoded_protobuf:len())

    local body = packed_prefix .. payload_len .. encoded_protobuf

    return body
end

return protobuf
