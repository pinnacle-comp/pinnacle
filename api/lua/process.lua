-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@diagnostic disable: redefined-local

local process = {}

---Spawn a process with an optional callback for its stdout, stderr, and exit information.
---
---`callback` has the following parameters:
--- - `stdout`: The process's stdout printed this line.
--- - `stderr`: The process's stderr printed this line.
--- - `exit_code`: The process exited with this code.
--- - `exit_msg`: The process exited with this message.
---@param command string|string[] The command as one whole string or a table of each of its arguments
---@param callback fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil)? A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process.spawn(command, callback)
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
--- - `stdout`: The process's stdout printed this line.
--- - `stderr`: The process's stderr printed this line.
--- - `exit_code`: The process exited with this code.
--- - `exit_msg`: The process exited with this message.
---
---`spawn_once` checks for the process using `pgrep`. If your system doesn't have `pgrep`, this won't work properly.
---@param command string|string[] The command as one whole string or a table of each of its arguments
---@param callback fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil)? A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process.spawn_once(command, callback)
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
    process.spawn(command, callback)
end

return process
