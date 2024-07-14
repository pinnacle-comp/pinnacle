-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local logging = require("logging")

---@class pinnacle.Log
---@field debug function
---@field info function
---@field warn function
---@field error function
---@field fatal function
local log = {}

local console_logger = require("logging.console")({
    logPattern = "%level %message\n",
    logPatterns = {
        [logging.ERROR] = "%level %message (at %source)\n",
    },
})

setmetatable(log, {
    __index = console_logger,
})

log.debug = function(_, ...)
    console_logger:debug(...)
    io.flush()
end

log.info = function(_, ...)
    console_logger:info(...)
    io.flush()
end

log.warn = function(_, ...)
    console_logger:warn(...)
    io.flush()
end

log.error = function(_, ...)
    console_logger:error(...)
    io.flush()
end

log.fatal = function(_, ...)
    console_logger:fatal(...)
    io.flush()
end

return log
