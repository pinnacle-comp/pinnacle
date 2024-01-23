-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---The protobuf absolute path prefix
local prefix = "pinnacle.process." .. require("pinnacle").version .. "."
local service = prefix .. "ProcessService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) ProcessServiceMethod
local rpc_types = {
    Spawn = {
        response_type = "SpawnResponse",
    },
    SetEnv = {},
}

---Build GrpcRequestParams
---@param method ProcessServiceMethod
---@param data table
---@return GrpcRequestParams
local function build_grpc_request_params(method, data)
    local req_type = rpc_types[method].request_type
    local resp_type = rpc_types[method].response_type

    ---@type GrpcRequestParams
    return {
        service = service,
        method = method,
        request_type = req_type and prefix .. req_type or prefix .. method .. "Request",
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---@nodoc
---@class ProcessModule
local process = {}

---Process management.
---
---This module provides utilities to spawn processes and capture their output.
---@class Process
---@field private config_client Client
local Process = {}

---@param config_client Client
---@param args string[]
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }?
---@param once boolean
local function spawn_inner(config_client, args, callbacks, once)
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

    config_client:server_streaming_request(
        build_grpc_request_params("Spawn", {
            args = args,
            once = once,
            has_callback = callbacks ~= nil,
        }),
        callback
    )
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
---@param args string | string[] The program arguments; a string instead of an array should be for only 1 argument
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }? Callbacks that will be run whenever the program outputs to stdout, stderr, or exits.
function Process:spawn(args, callbacks)
    if type(args) == "string" then
        args = { args }
    end

    spawn_inner(self.config_client, args, callbacks, false)
end

---Like `Process:spawn` but will only spawn the program if it isn't already running.
---
---@param args string | string[]
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }?
---
---@see Process.spawn
function Process:spawn_once(args, callbacks)
    if type(args) == "string" then
        args = { args }
    end

    spawn_inner(self.config_client, args, callbacks, true)
end

---Set an environment variable for the compositor.
---This will cause any future spawned processes to have this environment variable.
---
---@param key string The environment variable key
---@param value string The environment variable value
function Process:set_env(key, value)
    self.config_client:unary_request(build_grpc_request_params("SetEnv", {
        key = key,
        value = value,
    }))
end

function process.new(config_client)
    ---@type Process
    local self = { config_client = config_client }
    setmetatable(self, { __index = Process })
    return self
end

return process
