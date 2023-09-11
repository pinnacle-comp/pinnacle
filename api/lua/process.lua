-- SPDX-License-Identifier: GPL-3.0-or-later

---@diagnostic disable: redefined-local

---Process management.
---
---This module provides utilities to spawn processes and capture their output.
---@class ProcessModule
local process_module = {}

---Spawn a process with an optional callback for its stdout, stderr, and exit information.
---
---`callback` has the following parameters:
---
--- - `stdout` - The process's stdout printed this line.
--- - `stderr` - The process's stderr printed this line.
--- - `exit_code` - The process exited with this code.
--- - `exit_msg` - The process exited with this message.
---@param command string|string[] The command as one whole string or a table of each of its arguments
---@param callback fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil)? A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process_module.spawn(command, callback)
    ---@type integer|nil
    local callback_id = nil

    if callback ~= nil then
        ---@param args Args
        table.insert(CallbackTable, function(args)
            local args = args.Spawn or {} -- don't know if the `or {}` is necessary
            callback(args.stdout, args.stderr, args.exit_code, args.exit_msg)
        end)
        callback_id = #CallbackTable
    end

    local command_arr = {}
    if type(command) == "string" then
        for i in string.gmatch(command, "%S+") do
            table.insert(command_arr, i)
        end
    else
        command_arr = command
    end

    SendMsg({
        Spawn = {
            command = command_arr,
            callback_id = callback_id,
        },
    })
end

---Spawn a process only if it isn't already running, with an optional callback for its stdout, stderr, and exit information.
---
---`callback` has the following parameters:
---
--- - `stdout`: The process's stdout printed this line.
--- - `stderr`: The process's stderr printed this line.
--- - `exit_code`: The process exited with this code.
--- - `exit_msg`: The process exited with this message.
---
---`spawn_once` checks for the process using `pgrep`. If your system doesn't have `pgrep`, this won't work properly.
---@param command string|string[] The command as one whole string or a table of each of its arguments
---@param callback fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil)? A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process_module.spawn_once(command, callback)
    local proc = ""
    if type(command) == "string" then
        proc = command:match("%S+")
    else
        proc = command[1]
    end

    ---@type string
    local procs = io.popen("pgrep -f " .. proc):read("*a")
    if procs:len() ~= 0 then -- if process exists, return
        return
    end
    process_module.spawn(command, callback)
end

---Set an environment variable for Pinnacle. All future processes spawned will have this env set.
---
---Note that this will only set the variable for Pinnacle the compositor, not the running Lua config process.
---If you need to set an environment variable for this config, place them in the `metaconfig.toml` file instead.
---
---### Example
---```lua
---process.set_env("MOZ_ENABLE_WAYLAND", "1")
---```
---@param key string
---@param value string
function process_module.set_env(key, value)
    SendMsg({
        SetEnv = {
            key = key,
            value = value,
        },
    })
end

return process_module
