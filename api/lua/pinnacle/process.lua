-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local condition = require("cqueues.condition")

---The standard input of a spawned process.
---@class pinnacle.process.ChildStdin
---@field write fun(self: self, ...) Same as `file:write(...)`

---The standard output of a spawned process.
---@class pinnacle.process.ChildStdout
---@field lines fun(self: self, ...) Same as `file:lines(...)`
---@field read fun(self: self, ...) Same as `file:read(...)`

---The standard error of a spawned process.
---@class pinnacle.process.ChildStderr
---@field lines fun(self: self, ...) Same as `file:lines(...)`
---@field read fun(self: self, ...) Same as `file:read(...)`

---The result of spawning a command.
---@class pinnacle.process.Child
---The pid of the spawned command.
---@field pid integer
---This process's standard input, if any.
---
---This will only exist if `pipe_stdin` was set on the `Command` before spawning.
---@field stdin pinnacle.process.ChildStdin?
---This process's standard output, if any.
---
---This will only exist if `pipe_stdout` was set on the `Command` before spawning.
---@field stdout pinnacle.process.ChildStdout?
---This process's standard error, if any.
---
---This will only exist if `pipe_stderr` was set on the `Command` before spawning.
---@field stderr pinnacle.process.ChildStderr?
local Child = {}

local child_module = {}

---Convert a Child to a string
---
---@param child pinnacle.process.Child
---@return string
local function child_tostring(child)
    return "Child{pid=" .. child.pid .. "}"
end

---@param child pinnacle.process.Child
---
---@return pinnacle.process.Child
function child_module.new_child(child)
    setmetatable(child, {
        __index = Child,
        __gc = function(self)
            client.loop:wrap(function()
                self:wait()
            end)
        end,
        __tostring = child_tostring,
    })
    return child
end

---A command representing a to-be-spawned process.
---@class pinnacle.process.Command
---@field private cmd string | string[]
---@field private shell_cmd string[]?
---@field private envs table<string, string>?
---@field private unique boolean?
---@field private once boolean?
---@field private pipe_stdin boolean?
---@field private pipe_stdout boolean?
---@field private pipe_stderr boolean?
local Command = {}

---Options for a command.
---@class pinnacle.process.CommandOpts
---@field cmd string | string[] The command to be run
---An optional shell command that will be prefixed with `cmd`.
---Use this to spawn something with a shell.
---@field shell_cmd string[]?
---Any environment variables that should be set for the spawned process.
---@field envs table<string, string>?
---Prevents the spawn from occurring if the process is already running.
---@field unique boolean?
---Causes the command to only spawn the process if it hasn't been spawned before within the
---lifetime of the compositor.
---@field once boolean?
---Sets up a pipe to allow the config to write to the process's stdin.
---
---The pipe will be available through the spawned child's `stdin`.
---@field pipe_stdin boolean?
---Sets up a pipe to allow the config to read from the process's stdout.
---
---The pipe will be available through the spawned child's `stdout`.
---@field pipe_stdout boolean?
---Sets up a pipe to allow the config to read from the process's stderr.
---
---The pipe will be available through the spawned child's `stderr`.
---@field pipe_stderr boolean?

---Spawns this process, returning a `Child` that contains the process's standard IO if successful.
---
---@return pinnacle.process.Child? # A child with the process's standard IO, or `nil` if the process failed to spawn or doesn't exist.
function Command:spawn()
    local response, err = client:pinnacle_process_v1_ProcessService_Spawn({
        cmd = type(self.cmd) == "string" and { self.cmd } or self.cmd,
        shell_cmd = self.shell_cmd,
        unique = self.unique,
        once = self.once,
        envs = self.envs,
        pipe_stdin = self.pipe_stdin,
        pipe_stdout = self.pipe_stdout,
        pipe_stderr = self.pipe_stderr,
    })

    if err then
        log.error(err)
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
---@return { exit_code: integer?, exit_msg: string? } # The exit status of the process.
function Child:wait()
    local condvar = condition.new()

    local ret = {}

    local err = client:pinnacle_process_v1_ProcessService_WaitOnSpawn({
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
---@param ... string The arguments of the command.
---
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child? # A child with the process's standard IO, or `nil` if the process failed to spawn or doesn't exist.
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
---@param ... string The arguments of the command.
---
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child? # A child with the process's standard IO, or `nil` if the process failed to spawn or doesn't exist.
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
---@param ... string The arguments of the command.
---
---@overload fun(cmd: string[]): pinnacle.process.Child?
---
---@return pinnacle.process.Child? # A child with the process's standard IO, or `nil` if the process failed to spawn or doesn't exist.
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
---@param cmd pinnacle.process.CommandOpts Options for the command.
---
---@return pinnacle.process.Command # An object that allows you to spawn this command.
---@nodiscard
function process.command(cmd)
    setmetatable(cmd, { __index = Command })
    return cmd --[[@as pinnacle.process.Command]]
end

---Adds an environment variable that all newly spawned processes will inherit.
---
---@param key string The environment variable's key.
---@param value string The environment variable's value.
function process.set_env(key, value)
    local _, err = client:pinnacle_process_v1_ProcessService_SetEnv({
        key = key,
        value = value,
    })

    if err then
        log.error(err)
    end
end

return process
