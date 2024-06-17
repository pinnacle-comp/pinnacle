-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")
local pinnacle_service = require("pinnacle.grpc.defs").pinnacle.v0alpha1.PinnacleService

---The entry point to configuration.
---
---This module contains the `setup` function, which is how you'll access all the ways to configure Pinnacle.
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
    ---@type Util
    util = require("pinnacle.util"),
    ---@type Layout
    layout = require("pinnacle.layout"),
    ---@type Render
    render = require("pinnacle.render"),
    ---@type pinnacle.Snowcap
    snowcap = require("pinnacle.snowcap"),
}

---Quit Pinnacle.
function pinnacle.quit()
    client.unary_request(pinnacle_service.Quit, {})
end

---Reload the active config.
function pinnacle.reload_config()
    client.unary_request(pinnacle_service.ReloadConfig, {})
end

---Setup a Pinnacle config.
---
---You must pass in a function that takes in the `Pinnacle` table. This table is how you'll access the other config modules.
---
---You can also `require` the other modules. Just be sure not to call any of their functions outside this
---setup function.
---
---If you want to run a function with the config without blocking at the end, see `Pinnacle.run`.
---
---@param config_fn fun(pinnacle: Pinnacle)
---
---@see Pinnacle.run
function pinnacle.setup(config_fn)
    require("pinnacle.grpc.protobuf").build_protos()
    local success, snowcap = pcall(require, "snowcap")
    if success then
        snowcap.init()
    end

    -- Make Snowcap use Pinnacle's cqueues loop
    require("snowcap.grpc.client").loop = client.loop

    -- This function ensures a config won't run forever if Pinnacle is killed
    -- and doesn't kill configs on drop.
    client.loop:wrap(function()
        while true do
            require("cqueues").sleep(60)
            local success, err, errno = client.conn:ping(10)
            if not success then
                print("Compositor ping failed:", err, errno)
                os.exit(1)
            end
        end
    end)

    config_fn(pinnacle)

    local success, err = client.loop:loop()
    if not success then
        print(err)
    end
end

---Run a function with the Pinnacle API.
---
---If you are writing a config, use `Pinnacle.setup` instead.
---
---Like `Pinnacle.setup`, this function takes in a function that takes in the `Pinnacle` table.
---This allows you to run anything that `setup` can run.
---
---*Unlike* `setup`, this will **not** listen to the compositor for incoming key presses, signals, and the like.
---This means that this function will not block and can be used to integrate with external applications
---like taskbars and widget systems like eww, but it will not allow you to set usable keybinds or
---call signal callbacks. This is useful for things like querying compositor information for outputs and
---windows.
---
---@param run_fn fun(pinnacle: Pinnacle)
function pinnacle.run(run_fn)
    require("pinnacle.grpc.protobuf").build_protos()
    local success, snowcap = pcall(require, "snowcap")
    if success then
        snowcap.init()
    end

    run_fn(pinnacle)
end

return pinnacle
