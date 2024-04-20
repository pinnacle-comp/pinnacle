-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")
local process_service = require("pinnacle.grpc.defs").pinnacle.process.v0alpha1.ProcessService

---Process management.
---
---This module provides utilities to spawn processes and capture their output.
---@class Process
local process = {}

---@param args string[]
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }?
---@param once boolean
local function spawn_inner(args, callbacks, once)
    local callback = function() end

    if callbacks then
        callback = function(response)
            if callbacks.stdout and response.stdout then
                callbacks.stdout(response.stdout)
            end
            if callbacks.stderr and response.stderr then
                callbacks.stderr(response.stderr)
            end
            if callbacks.exit and (response.exit_code or response.exit_message) then
                callbacks.exit(response.exit_code, response.exit_message)
            end
        end
    end

    client.server_streaming_request(process_service.Spawn, {
        args = args,
        once = once,
        has_callback = callbacks ~= nil,
    }, callback)
end

---Spawn a program with optional callbacks for its stdout, stderr, and exit information.
---
---`callbacks` is an optional table with the following optional fields:
--- - `stdout`: function(line: string)
--- - `stderr`: function(line: string)
--- - `exit`:   function(code: integer, msg: string)
---
---Note: if `args` is a string then it will be wrapped in a table and sent to the compositor.
---If you need multiple arguments, use a string array instead.
---
---Note 2: If you spawn a window before tags are added it will spawn without any tags and
---won't be displayed in the compositor. TODO: Do what awesome does and display on all tags instead
---
---### Example
---```lua
---Process.spawn("alacritty")
---
--- -- With a table of arguments
---Process.spawn({ "bash", "-c", "echo hello" })
---
--- -- With callbacks
---Process.spawn("alacritty", {
---    stdout = function(line)
---        print(line)
---    end,
---    -- You can ignore callbacks you don't need
---    exit = function(code, msg)
---        print("exited with code", code)
---        print("exited with msg", msg)
---    end,
---})
---```
---
---@param args string | string[] The program arguments; a string instead of an array should be for only 1 argument
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }? Callbacks that will be run whenever the program outputs to stdout, stderr, or exits.
function process.spawn(args, callbacks)
    if type(args) == "string" then
        args = { args }
    end

    spawn_inner(args, callbacks, false)
end

---Like `Process.spawn` but will only spawn the program if it isn't already running.
---
---@param args string | string[]
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }?
---
---@see Process.spawn
function process.spawn_once(args, callbacks)
    if type(args) == "string" then
        args = { args }
    end

    spawn_inner(args, callbacks, true)
end

---Set an environment variable for the compositor.
---This will cause any future spawned processes to have this environment variable.
---
---### Example
---```lua
---Process.set_env("ENV_NAME", "the value")
---```
---
---@param key string The environment variable key
---@param value string The environment variable value
function process.set_env(key, value)
    client.unary_request(process_service.SetEnv, {
        key = key,
        value = value,
    })
end

return process
