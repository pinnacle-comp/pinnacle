local pb = require("pb")

local protobuf = {}

function protobuf.build_protos()
    local version = "v0alpha1"
    local proto_file_paths = {
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/tag/" .. version .. "/tag.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/input/" .. version .. "/input.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/input/libinput/" .. version .. "/libinput.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/" .. version .. "/pinnacle.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/output/" .. version .. "/output.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/process/" .. version .. "/process.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/window/" .. version .. "/window.proto",
        "/home/jason/projects/pinnacle/api/protocol/pinnacle/window/rules/" .. version .. "/rules.proto",
    }

    local cmd = "protoc --descriptor_set_out=/tmp/pinnacle.pb --proto_path=/home/jason/projects/pinnacle/api/protocol/ "

    for _, file_path in pairs(proto_file_paths) do
        cmd = cmd .. file_path .. " "
    end

    local proc = assert(io.popen(cmd), "protoc is not installed")
    local _ = proc:read("a")
    proc:close()

    local pinnacle_pb = assert(io.open("/tmp/pinnacle.pb", "r"), "no pb file generated")
    local pinnacle_pb_data = pinnacle_pb:read("a")
    pinnacle_pb:close()

    assert(pb.load(pinnacle_pb_data), "failed to load .pb file")
end

return protobuf
