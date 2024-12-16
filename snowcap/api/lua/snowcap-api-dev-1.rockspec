package = "snowcap-api"
version = "dev-1"
source = {
    url = "*** please add URL for source tarball, zip or repository here ***",
}
description = {
    homepage = "*** please enter a project homepage ***",
    license = "MPL 2.0",
}
dependencies = {
    "lua >= 5.2",
    "cqueues ~> 20200726",
    "http ~> 0.4",
    "lua-protobuf ~> 0.5",
    "compat53 ~> 0.13",
    "lualogging ~> 1.8.2",

    -- Run just install
    "lua-grpc-client >= dev-1",
}
build = {
    type = "builtin",
    modules = {
        snowcap = "snowcap.lua",
        ["snowcap.grpc.client"] = "snowcap/grpc/client.lua",
        ["snowcap.grpc.protobuf"] = "snowcap/grpc/protobuf.lua",
        ["snowcap.grpc.defs"] = "snowcap/grpc/defs.lua",
        ["snowcap.input"] = "snowcap/input.lua",
        ["snowcap.input.keys"] = "snowcap/input/keys.lua",
        ["snowcap.widget"] = "snowcap/widget.lua",
        ["snowcap.layer"] = "snowcap/layer.lua",
        ["snowcap.util"] = "snowcap/util.lua",
        ["snowcap.log"] = "snowcap/log.lua",
    },
}
