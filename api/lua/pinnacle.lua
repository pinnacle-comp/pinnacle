-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local pinnacle_service = require("pinnacle.grpc.defs").pinnacle.v1.PinnacleService

---The entry point to configuration.
---
---This module contains the `setup` function, which is where you'll put all of your config in.
---It also contains general compositor actions like `quit` and `reload_config`.
---
---@class Pinnacle
local pinnacle = {}

---Quits Pinnacle.
function pinnacle.quit()
    local _, err = client:unary_request(pinnacle_service.Quit, {})

    if err then
        log:error(err)
    end
end

---Reloads the active config.
function pinnacle.reload_config()
    local _, err = client:unary_request(pinnacle_service.ReloadConfig, {})

    if err then
        log:error(err)
    end
end

---Gets the currently running backend.
---
---@return "tty" | "window" `"tty"` if Pinnacle is running in a tty, or `"window"` if it's running in a nested window
function pinnacle.backend()
    local response, err = client:unary_request(pinnacle_service.Backend, {})

    if err then
        log:error(err)
        -- TODO: possibly panic here; a nil index error will be thrown after this anyway
    end

    ---@cast response pinnacle.v1.BackendResponse

    local defs = require("pinnacle.grpc.defs")

    if response.backend == defs.pinnacle.v1.Backend.BACKEND_WINDOW then
        return "window"
    else
        return "tty"
    end
end

---Initializes the protobuf backend and connects to Pinnacle's gRPC socket.
---
---If the Snowcap Lua API is installed and Snowcap is running, this will also setup Snowcap and
---connect to its socket as well.
function pinnacle.init()
    require("pinnacle.grpc.protobuf").build_protos()

    require("pinnacle.grpc.client").connect()

    local success, snowcap = pcall(require, "snowcap")
    if success then
        if pcall(snowcap.init) then
            pinnacle.snowcap = require("pinnacle.snowcap")

            -- Make Snowcap use Pinnacle's cqueues loop
            require("snowcap.grpc.client").client.loop = client.loop
        end
    end
end

---Sets up a Pinnacle config.
---
---This receives a function that contains your config.
---
---If you want to run a function with the config without blocking, see `Pinnacle.run`.
---
---@param config_fn fun()
---
---@see Pinnacle.run
function pinnacle.setup(config_fn)
    pinnacle.init()

    -- This ensures a config won't run forever if Pinnacle is killed
    -- and doesn't kill configs on drop.
    client.loop:wrap(function()
        while true do
            require("cqueues").sleep(30)
            local success, err, errno = client.conn:ping(10)
            if not success then
                error(
                    "compositor ping failed: err = "
                        .. tostring(err)
                        .. ", errno = "
                        .. tostring(errno)
                )
            end
        end
    end)

    client.loop:wrap(config_fn)

    local success, err = client.loop:loop()
    if not success then
        error("loop errored: " .. tostring(err))
    end
end

---Runs a function with the Pinnacle API.
---
---If you are writing a config, use `Pinnacle.setup` instead.
---
---This receives a function that runs anything in this API.
---However, it will not block to receive compositor events.
---
---@param run_fn fun()
function pinnacle.run(run_fn)
    pinnacle.init()

    run_fn()
end

return pinnacle
