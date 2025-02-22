-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@class snowcap.Log
local log = {}

---@param level "DEBUG" | "INFO" | "WARN" | "ERROR"
---@param msg string
local function print_log(level, msg)
    local source = ""
    if level == "ERROR" then
        local debuginfo_callsite_parent_func = debug.getinfo(3)
        local debuginfo_callsite_log = debug.getinfo(2)
        local callsite_func = "main"
        if debuginfo_callsite_parent_func then
            callsite_func = debuginfo_callsite_parent_func.name or "main"
        end
        assert(debuginfo_callsite_log)
        local callsite_line = debug.getinfo(3, "l").currentline
        source = " (in function `" .. callsite_func .. "`, line " .. tostring(callsite_line) .. ")"
    end

    print(level .. " " .. msg .. source)
end

function log.info(msg)
    print_log("INFO", msg)
end

function log.debug(msg)
    print_log("DEBUG", msg)
end

function log.warn(msg)
    print_log("WARN", msg)
end

function log.error(msg)
    print_log("ERROR", msg)
end

return log
