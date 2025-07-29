-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---Logging utilities.
---@class pinnacle.Log
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

---Prints an INFO message.
function log.info(...)
    local args = { ... }
    local msg = tostring(table.remove(args, 1))

    for _, v in ipairs(args) do
        msg = msg .. " " .. tostring(v)
    end

    print_log("INFO", msg)
end

---Prints a DEBUG message.
function log.debug(...)
    local args = { ... }
    local msg = tostring(table.remove(args, 1))

    for _, v in ipairs(args) do
        msg = msg .. " " .. tostring(v)
    end

    print_log("DEBUG", msg)
end

---Prints a WARN message.
function log.warn(...)
    local args = { ... }
    local msg = tostring(table.remove(args, 1))

    for _, v in ipairs(args) do
        msg = msg .. " " .. tostring(v)
    end

    print_log("WARN", msg)
end

---Prints an ERROR message along with file, line, and column information.
function log.error(...)
    local args = { ... }
    local msg = tostring(table.remove(args, 1))

    for _, v in ipairs(args) do
        msg = msg .. " " .. tostring(v)
    end

    print_log("ERROR", msg)
end

return log
