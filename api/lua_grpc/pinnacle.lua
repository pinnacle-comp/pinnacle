local cqueues = require("cqueues")

local client = require("pinnacle.grpc.client")

---@class PinnacleModule
local pinnacle = {
    version = "v0alpha1",
}

---@class Pinnacle
---@field private config_client Client
---@field input Input
---@field output Output
---@field process Process
---@field tag Tag
---@field window Window
local Pinnacle = {}

function Pinnacle:quit()
    self.config_client:unary_request({
        service = "pinnacle.v0alpha1.PinnacleService",
        method = "Quit",
        request_type = "pinnacle.v0alpha1.QuitRequest",
        data = {},
    })
end

---Setup Pinnacle.
---@param config_fn fun(pinnacle: Pinnacle)
function pinnacle.setup(config_fn)
    require("pinnacle.grpc.protobuf").build_protos()

    local loop = cqueues.new()

    local config_client = client.new(loop)

    ---@type Pinnacle
    local self = {
        config_client = config_client,
        input = require("pinnacle.input").new(config_client),
        process = require("pinnacle.process").new(config_client),
        window = require("pinnacle.window").new(config_client),
        output = require("pinnacle.output").new(config_client),
        tag = require("pinnacle.tag").new(config_client),
    }
    setmetatable(self, { __index = Pinnacle })

    config_fn(self)

    loop:loop()
end

return pinnacle