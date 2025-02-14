-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local process_service = require("pinnacle.grpc.defs").pinnacle.process.v1.ProcessService
local condition = require("cqueues.condition")

---@class pinnacle.process.ChildStdin
---@field write fun(...) Same as `file:write(...)`

---@class pinnacle.process.ChildStdout
---@field lines fun(...) Same as `file:lines(...)`
---@field read fun(...) Same as `file:read(...)`

---@class pinnacle.process.ChildStderr
---@field lines fun(...) Same as `file:lines(...)`
---@field read fun(...) Same as `file:read(...)`

---@class pinnacle.process.Child
---@field pid integer
---@field stdin pinnacle.process.ChildStdin?
---@field stdout pinnacle.process.ChildStdout?
---@field stderr pinnacle.process.ChildStderr?
local Child = {}

local child_module = {}

---@param child pinnacle.process.Child
---
---@return pinnacle.process.Child
function child_module.new_child(child)
    setmetatable(child, { __index = Child })
    return child
end

---A command representing a to-be-spawned process.
---@class pinnacle.process.Command
---@field private cmd string | string[]
---@field private shell_cmd string[]?
---@field private envs table<string, string>?
---@field private unique boolean?
---@field private once boolean?
local Command = {}

---Options for a command.
---@class pinnacle.process.CommandOpts
---@field cmd string | string[] The command to be run
---An optional shell command that will be prefixed with `cmd`.
---Use this to spawn something with a shell.
---@field shell_cmd string[]?
---Any environment variables that should be set for the spawned process.
---@field envs table<string, string>?
---Causes the spawn to fizzle if the process is already running.
---@field unique boolean?
---Causes the command to only spawn the process if it hasn't been spawned before within the
---lifetime of the compositor.
---@field once boolean?

---Spawns this process, returning a `Child` that contains the process's standard IO if successful.
---
---@return pinnacle.process.Child?
function Command:spawn()
    local response, err = client:unary_request(process_service.Spawn, {
        cmd = type(self.cmd) == "string" and { self.cmd } or self.cmd,
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

---Waits for this child process to exit.
---
---This will block the calling thread.
---
---Returns the exit status of the process.
---
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

---Runs a function with every line of the child process's standard output.
---
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

---Runs a function with every line of the child process's standard error.
---
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
---@class pinnacle.process
local process = {}

---Spawns a process, returning a `Child` with the process's standard IO if successful.
---
---Receives the arguments of the command to be spawned, either as varargs or as a table.
---
---For more control over the spawn, use `Process.command` instead.
---
---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
---
---@see pinnacle.process.Process.command A way to spawn processes with more control.
function process.spawn(...)
    local cmd = { ... }
    if cmd[1] and type(cmd[1]) == "table" then
        cmd = cmd[1]
    end

    return process
        .command({
            cmd = cmd,
        })
        :spawn()
end

---Spawns a process if it hasn't been spawned before,
---returning a `Child` with the process's standard IO if successful.
---
---Receives the arguments of the command to be spawned, either as varargs or as a table.
---
---For more control over the spawn, use `Process.command` instead.
---
---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
---
---@see pinnacle.process.Process.command A way to spawn processes with more control.
function process.spawn_once(...)
    local cmd = { ... }
    if cmd[1] and type(cmd[1]) == "table" then
        cmd = cmd[1]
    end

    return process
        .command({
            cmd = cmd,
            once = true,
        })
        :spawn()
end

---Spawns a process if it isn't already running,
---returning a `Child` with the process's standard IO if successful.
---
---Receives the arguments of the command to be spawned, either as varargs or as a table.
---
---For more control over the spawn, use `Process.command` instead.
---
---@param ... string
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child?
---
---@see pinnacle.process.Process.command A way to spawn processes with more control.
function process.spawn_unique(...)
    local cmd = { ... }
    if cmd[1] and type(cmd[1]) == "table" then
        cmd = cmd[1]
    end

    return process
        .command({
            cmd = cmd,
            unique = true,
        })
        :spawn()
end

---Creates a `Command` from the given options.
---
---A `Command` represents a to-be-spawned process.
---
---@param cmd pinnacle.process.CommandOpts
---
---@return pinnacle.process.Command
---@nodiscard
function process.command(cmd)
    setmetatable(cmd, { __index = Command })
    return cmd --[[@as pinnacle.process.Command]]
end

return process
