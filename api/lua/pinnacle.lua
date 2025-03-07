-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- INFO: In order to not have to package the snowcap API separately and avoid
-- packaging issues down the road, we're symlinking the API under the `pinnacle`
-- directory. We add a searcher here that checks for requires of the snowcap API
-- and points them to the symlinked directory.
--
-- TODO: Remove this when snowcap is stable enough to become its own project
local path_searcher = package.searchers[2]
-- Insert before the actual package.path searcher so it takes priority
table.insert(package.searchers, 2, function(libname)
    if libname:match("snowcap") then
        return path_searcher("pinnacle.snowcap." .. libname)
    else
        return path_searcher(libname)
    end
end)

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client

---The entry point to configuration.
---
---This module contains the `setup` function, which is where you'll put all of your config in.
---It also contains general compositor actions like `quit` and `reload_config`.
---
---@class pinnacle
local pinnacle = {}

---Quits Pinnacle.
function pinnacle.quit()
    local _, err = client:pinnacle_v1_PinnacleService_Quit({})

    if err then
        log.error(err)
    end
end

---Reloads the active config.
function pinnacle.reload_config()
    local _, err = client:pinnacle_v1_PinnacleService_ReloadConfig({})

    if err then
        log.error(err)
    end
end

---Gets the currently running backend.
---
---@return "tty" | "window" `"tty"` if Pinnacle is running in a tty, or `"window"` if it's running in a nested window
function pinnacle.backend()
    local response, err = client:pinnacle_v1_PinnacleService_Backend({})

    if err then
        log.error(err)
        -- TODO: possibly panic here; a nil index error will be thrown after this anyway
    end

    assert(response)

    local defs = require("pinnacle.grpc.defs")

    if response.backend == defs.pinnacle.v1.Backend.BACKEND_WINDOW then
        return "window"
    else
        return "tty"
    end
end

---Sets whether or not xwayland clients should scale themselves.
---
---If `true`, xwayland clients will be told they are on an output with a larger or smaller size than
---normal then rescaled to replicate being on an output with a scale of 1.
---
---Xwayland clients that support DPI scaling will scale properly, leading to crisp and correct scaling
---with fractional output scales. Those that don't, like `xterm`, will render as if they are on outputs
---with scale 1, and their scale will be slightly incorrect on outputs with fractional scale.
---
---Results may vary if you have multiple outputs with different scales.
---
---@param should_self_scale boolean
function pinnacle.set_xwayland_self_scaling(should_self_scale)
    local _, err = client:pinnacle_v1_PinnacleService_SetXwaylandClientSelfScale({
        self_scale = should_self_scale,
    })

    if err then
        log.error(err)
    end
end

---Initializes the protobuf backend and connects to Pinnacle's gRPC socket.
---
---If the Snowcap Lua API is installed and Snowcap is running, this will also setup Snowcap and
---connect to its socket as well.
function pinnacle.init()
    require("pinnacle.grpc.protobuf").build_protos()
    require("pinnacle.grpc.client").connect()
end

---Sets up a Pinnacle config.
---
---This receives a function that contains your config.
---
---If you want to run a function with the config without blocking, see `Pinnacle.run`.
---
---@param config_fn fun()
---
---@see pinnacle.run
function pinnacle.setup(config_fn)
    pinnacle.init()

    require("snowcap.grpc.client").client.loop = client.loop

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
---
---@see pinnacle.setup
function pinnacle.run(run_fn)
    pinnacle.init()

    run_fn()
end

return pinnacle
