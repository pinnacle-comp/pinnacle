-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")

---The entry point to configuration.
---
---This module contains one function: `setup`, which is how you'll access all the ways to configure Pinnacle.
---@class Pinnacle
local pinnacle = {
    ---@type Input
    input = require("pinnacle.input"),
    ---@type Tag
    tag = require("pinnacle.tag"),
    ---@type Output
    output = require("pinnacle.output"),
    ---@type Window
    window = require("pinnacle.window"),
    ---@type Process
    process = require("pinnacle.process"),
}

---Quit Pinnacle.
function pinnacle.quit()
    client.unary_request({
        service = "pinnacle.v0alpha1.PinnacleService",
        method = "Quit",
        request_type = "pinnacle.v0alpha1.QuitRequest",
        data = {},
    })
end

---Setup Pinnacle.
---
---You must pass in a function that takes in the `Pinnacle` table. This table is how you'll access the other config modules.
---
---You can also `require` the other modules. Just be sure not to call any of their functions outside this
---setup function.
---
---@param config_fn fun(pinnacle: Pinnacle)
function pinnacle.setup(config_fn)
    require("pinnacle.grpc.protobuf").build_protos()

    config_fn(pinnacle)

    client.loop:loop()
end

return pinnacle
