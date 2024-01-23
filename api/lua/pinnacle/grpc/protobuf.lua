-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local pb = require("pb")

local protobuf = {}

local PINNACLE_PROTO_DIR = os.getenv("PINNACLE_PROTO_DIR")

function protobuf.build_protos()
    local version = "v0alpha1"
    local proto_file_paths = {
        PINNACLE_PROTO_DIR .. "/pinnacle/tag/" .. version .. "/tag.proto",
        PINNACLE_PROTO_DIR .. "/pinnacle/input/" .. version .. "/input.proto",
        PINNACLE_PROTO_DIR .. "/pinnacle/" .. version .. "/pinnacle.proto",
        PINNACLE_PROTO_DIR .. "/pinnacle/output/" .. version .. "/output.proto",
        PINNACLE_PROTO_DIR .. "/pinnacle/process/" .. version .. "/process.proto",
        PINNACLE_PROTO_DIR .. "/pinnacle/window/" .. version .. "/window.proto",
    }

    local cmd = "protoc --descriptor_set_out=/tmp/pinnacle.pb --proto_path=" .. PINNACLE_PROTO_DIR .. " "

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

return protobuf
