local cqueues = require("cqueues")

---@type ClientModule
local client = require("pinnacle.grpc.client")

---@class PinnacleModule
local pinnacle = {}

---@class Pinnacle
---@field private config_client Client
---@field private loop CqueuesLoop
---@field input Input
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
    ---@type Client
    local config_client = client.new(loop)

    ---@type Pinnacle
    local self = {
        config_client = config_client,
        loop = loop,
        input = require("pinnacle.input").new(config_client),
    }
    setmetatable(self, { __index = Pinnacle })

    config_fn(self)

    self.loop:loop()
end

return pinnacle
