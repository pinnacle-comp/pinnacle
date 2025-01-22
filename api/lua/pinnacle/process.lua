-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local process_service = require("pinnacle.grpc.defs").pinnacle.process.v1.ProcessService
local condition = require("cqueues.condition")
local thread = require("cqueues.thread")

---@class pinnacle.process.Child
---@field pid integer
---@field stdin Socket?
---@field stdout Socket?
---@field stderr Socket?
local Child = {}

local child_module = {}

---@param child pinnacle.process.Child
---
---@return pinnacle.process.Child
function child_module.new_child(child)
    setmetatable(child, { __index = Child })
    return child
end

---@class Command
---@field private cmd string[]
---@field private shell_cmd string[]?
---@field private envs table<string, string>?
---@field private unique boolean?
---@field private once boolean?
local Command = {}

---@class CommandOpts
---@field cmd string[]
---@field shell_cmd string[]?
---@field envs table<string, string>?
---@field unique boolean?
---@field once boolean?

---@return pinnacle.process.Child?
function Command:spawn()
    local response, err = client:unary_request(process_service.Spawn, {
        cmd = self.cmd,
        shell_cmd = self.shell_cmd,
        unique = self.unique,
        once = self.once,
        envs = self.envs,
    })

    if err then
        log:error(err)
        return nil
    end

    ---@cast response pinnacle.process.v1.SpawnResponse

    if not response then
        return nil
    end

    local data = response.spawn_data
    if not data then
        return nil
    end

    local fd_socket = require("cqueues.socket").connect({
        path = data.fd_socket_path,
    })

    local stdin, stdout, stderr

    if data.has_stdin then
        local _, sock_stdin, err = fd_socket:recvfd()
        stdin = sock_stdin
    end
    if data.has_stdout then
        local _, sock_stdout, err = fd_socket:recvfd()
        stdout = sock_stdout
    end
    if data.has_stderr then
        local _, sock_stderr, err = fd_socket:recvfd()
        stderr = sock_stderr
    end

    fd_socket:close()

    ---@type pinnacle.process.Child
    local child = {
        pid = data.pid,
        stdin = stdin,
        stdout = stdout,
        stderr = stderr,
    }

    return child_module.new_child(child)
end

---@return { exit_code: integer?, exit_msg: string? }
function Child:wait()
    local condvar = condition.new()

    local ret = {}

    local err = client:server_streaming_request(process_service.WaitOnSpawn, {
        pid = self.pid,
    }, function(response)
        ret.exit_code = response.exit_code
        ret.exit_msg = response.exit_msg
        condvar:signal()
    end)

    if err then
        return {}
    end

    while not condvar:wait() do
    end

    return ret
end

---@param on_line fun(line: string)
---
---@return self self This child for chaining
function Child:on_line_stdout(on_line)
    if not self.stdout then
        print("no stdout")
        return self
    end

    client.loop:wrap(function()
        for line in self.stdout:lines() do
            on_line(line)
        end
        self.stdout:close()
    end)

    return self
end

---@param on_line fun(line: string)
---
---@return self self This child for chaining
function Child:on_line_stderr(on_line)
    if not self.stderr then
        return self
    end

    client.loop:wrap(function()
        for line in self.stderr:lines() do
            on_line(line)
        end
        self.stderr:close()
    end)

    return self
end

---Process management.
---
---This module provides utilities to spawn processes and capture their output.
---@class Process
local process = {}

---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
function process.spawn(...)
    local cmd = { ... }
    if cmd[0] and type(cmd[0]) == "table" then
        cmd = cmd[0]
    end

    return process
        .command({
            cmd = cmd,
        })
        :spawn()
end

---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
function process.spawn_once(...)
    local cmd = { ... }
    if cmd[0] and type(cmd[0]) == "table" then
        cmd = cmd[0]
    end

    return process
        .command({
            cmd = cmd,
            once = true,
        })
        :spawn()
end

---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
function process.spawn_unique(...)
    local cmd = { ... }
    if cmd[0] and type(cmd[0]) == "table" then
        cmd = cmd[0]
    end

    return process
        .command({
            cmd = cmd,
            unique = true,
        })
        :spawn()
end

---@param cmd CommandOpts
---
---@return Command
---@nodiscard
function process.command(cmd)
    setmetatable(cmd, { __index = Command })
    return cmd --[[@as Command]]
end

return process
