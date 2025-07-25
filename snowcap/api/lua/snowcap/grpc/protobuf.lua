-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

pcall(require, "compat53")

local stat = require("posix.sys.stat").stat
local pb = require("pb")

local protobuf = {}

function protobuf.build_protos()
    local version = "v1"
    local proto_file_paths = {
        "snowcap/input/" .. version .. "/input.proto",
        "snowcap/layer/" .. version .. "/layer.proto",
        "snowcap/widget/" .. version .. "/widget.proto",
        "snowcap/decoration/" .. version .. "/decoration.proto",
        "google/protobuf/empty.proto",
    }

    local xdg_data_home = os.getenv("XDG_DATA_HOME") or (os.getenv("HOME") .. "/.local/share")
    local xdg_data_dirs = os.getenv("XDG_DATA_DIRS")

    ---@type string[]
    local search_dirs = { xdg_data_home }

    if xdg_data_dirs then
        for data_dir in xdg_data_dirs:gmatch("[^:]+") do
            table.insert(search_dirs, data_dir)
        end
    end

    local proto_dir = nil

    for _, dir in ipairs(search_dirs) do
        -- Currently nesting protobufs in the pinnacle files until I spin this off into its own project
        dir = dir .. "/pinnacle"
        if stat(dir .. "/snowcap/protobuf") then
            proto_dir = dir .. "/snowcap/protobuf"
            break
        end
    end

    assert(proto_dir, "could not find protobuf definitions directory")

    local cmd = "protoc --descriptor_set_out=/tmp/snowcap.pb --proto_path=" .. proto_dir .. " "

    for _, file_path in ipairs(proto_file_paths) do
        cmd = cmd .. file_path .. " "
    end

    local proc = assert(io.popen(cmd), "protoc is not installed")
    local _ = proc:read("a")
    proc:close()

    local snowcap_pb = assert(io.open("/tmp/snowcap.pb", "r"), "no pb file generated")
    local snowcap_pb_data = snowcap_pb:read("a")
    snowcap_pb:close()

    assert(pb.load(snowcap_pb_data), "failed to load .pb file")

    pb.option("enum_as_value")
end

---@nodoc
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

    local packed_prefix = string.pack("I1", 0)
    local payload_len = string.pack(">I4", encoded_protobuf:len())

    local body = packed_prefix .. payload_len .. encoded_protobuf

    return body
end

return protobuf
