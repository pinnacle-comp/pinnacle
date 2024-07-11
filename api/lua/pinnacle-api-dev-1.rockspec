package = "pinnacle-api"
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
    "lua-grpc-client >= dev-1",
}
build = {
    type = "builtin",
    modules = {
        pinnacle = "pinnacle.lua",
        ["pinnacle.grpc.client"] = "pinnacle/grpc/client.lua",
        ["pinnacle.grpc.protobuf"] = "pinnacle/grpc/protobuf.lua",
        ["pinnacle.grpc.defs"] = "pinnacle/grpc/defs.lua",
        ["pinnacle.input"] = "pinnacle/input.lua",
        ["pinnacle.input.keys"] = "pinnacle/input/keys.lua",
        ["pinnacle.output"] = "pinnacle/output.lua",
        ["pinnacle.process"] = "pinnacle/process.lua",
        ["pinnacle.tag"] = "pinnacle/tag.lua",
        ["pinnacle.window"] = "pinnacle/window.lua",
        ["pinnacle.util"] = "pinnacle/util.lua",
        ["pinnacle.signal"] = "pinnacle/signal.lua",
        ["pinnacle.layout"] = "pinnacle/layout.lua",
        ["pinnacle.render"] = "pinnacle/render.lua",
        ["pinnacle.snowcap"] = "pinnacle/snowcap.lua",
    },
}
