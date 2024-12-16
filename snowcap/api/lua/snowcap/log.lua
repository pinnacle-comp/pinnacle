-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local logging = require("logging")

---@class snowcap.Log
---@field debug function
---@field info function
---@field warn function
---@field error function
---@field fatal function
local log = {}

local log_patterns = logging.buildLogPatterns({
    [logging.ERROR] = "%level %message (at %source)",
}, "%level %message")

local console_logger = logging.new(function(self, level, message)
    print(
        logging.prepareLogMsg(
            log_patterns[level],
            logging.date(logging.defaultTimestampPattern()),
            level,
            message
        )
    )
    return true
end, logging.defaultLevel())

setmetatable(log, {
    __index = console_logger,
})

return log
