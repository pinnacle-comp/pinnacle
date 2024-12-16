-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

require("compat53")

local pb = require("pb")

local protobuf = {}

local SNOWCAP_PROTO_DIR = (os.getenv("XDG_DATA_HOME") or (os.getenv("HOME") .. "/.local/share"))
    .. "/snowcap/protobuf"

function protobuf.build_protos()
    local version = "v0alpha1"
    local proto_file_paths = {
        SNOWCAP_PROTO_DIR .. "/snowcap/input/" .. version .. "/input.proto",
        SNOWCAP_PROTO_DIR .. "/snowcap/layer/" .. version .. "/layer.proto",
        SNOWCAP_PROTO_DIR .. "/snowcap/widget/" .. version .. "/widget.proto",
        SNOWCAP_PROTO_DIR .. "/google/protobuf/empty.proto",
    }

    local cmd = "protoc --descriptor_set_out=/tmp/snowcap.pb --proto_path="
        .. SNOWCAP_PROTO_DIR
        .. " "

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
