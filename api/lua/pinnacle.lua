-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local cqueues = require("cqueues")

local client = require("pinnacle.grpc.client")

---The entry point to configuration.
---
---This module contains one function: `setup`, which is how you'll access all the ways to configure Pinnacle.
---@class PinnacleModule
local pinnacle = {
    version = "v0alpha1",
}

---The Pinnacle module.
---
---This module holds all the other configuration modules (Window, Input, etc.), and allows you to
---quit the compositor using the `quit` method.
---@class Pinnacle
---@field private config_client Client
---@field input Input
---@field output Output
---@field process Process
---@field tag Tag
---@field window Window
local Pinnacle = {}

---Quit Pinnacle.
function Pinnacle:quit()
    self.config_client:unary_request({
        service = "pinnacle.v0alpha1.PinnacleService",
        method = "Quit",
        request_type = "pinnacle.v0alpha1.QuitRequest",
        data = {},
    })
end

---Setup Pinnacle.
---
---You must pass in a function that takes in the `Pinnacle` module object. The module is how you'll access the other config modules.
---
---Note: All the config modules are object instantiations, and their methods require you to use the colon operator
---instead of the dot operator to call them.
---
---If you want to do a multi-file config, you should have other files return a function taking in necessary modules.
---Or you could cheat and stick the modules into globals :TrollFace:
---
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
