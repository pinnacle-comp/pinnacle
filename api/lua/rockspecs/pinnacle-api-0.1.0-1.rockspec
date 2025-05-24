package = "pinnacle-api"
version = "0.1.0"
source = {
    url = "git+https://github.com/pinnacle-comp/pinnacle",
    dir = "pinnacle/api/lua",
    tag = "v0.1.0",
}
description = {
    homepage = "https://github.com/pinnacle-comp/pinnacle",
    license = "MPL 2.0",
}
dependencies = {
    "lua >= 5.2",
    "cqueues ~> 20200726",
    "http ~> 0.4",
    "lua-protobuf ~> 0.5.2",
    "compat53 ~> 0.13",
    "luaposix ~> 36.3",
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
        ["pinnacle.input.libinput"] = "pinnacle/input/libinput.lua",
        ["pinnacle.output"] = "pinnacle/output.lua",
        ["pinnacle.process"] = "pinnacle/process.lua",
        ["pinnacle.tag"] = "pinnacle/tag.lua",
        ["pinnacle.window"] = "pinnacle/window.lua",
        ["pinnacle.util"] = "pinnacle/util.lua",
        ["pinnacle.signal"] = "pinnacle/signal.lua",
        ["pinnacle.layout"] = "pinnacle/layout.lua",
        ["pinnacle.render"] = "pinnacle/render.lua",
        ["pinnacle.snowcap"] = "pinnacle/snowcap.lua",
        ["pinnacle.log"] = "pinnacle/log.lua",
        ["pinnacle.debug"] = "pinnacle/debug.lua",

        -- scuff city
        ["pinnacle.snowcap.snowcap"] = "pinnacle/snowcap/snowcap.lua",
        ["pinnacle.snowcap.snowcap.grpc.client"] = "pinnacle/snowcap/snowcap/grpc/client.lua",
        ["pinnacle.snowcap.snowcap.grpc.protobuf"] = "pinnacle/snowcap/snowcap/grpc/protobuf.lua",
        ["pinnacle.snowcap.snowcap.grpc.defs"] = "pinnacle/snowcap/snowcap/grpc/defs.lua",
        ["pinnacle.snowcap.snowcap.input"] = "pinnacle/snowcap/snowcap/input.lua",
        ["pinnacle.snowcap.snowcap.input.keys"] = "pinnacle/snowcap/snowcap/input/keys.lua",
        ["pinnacle.snowcap.snowcap.widget"] = "pinnacle/snowcap/snowcap/widget.lua",
        ["pinnacle.snowcap.snowcap.layer"] = "pinnacle/snowcap/snowcap/layer.lua",
        ["pinnacle.snowcap.snowcap.util"] = "pinnacle/snowcap/snowcap/util.lua",
        ["pinnacle.snowcap.snowcap.log"] = "pinnacle/snowcap/snowcap/log.lua",
    },
}
