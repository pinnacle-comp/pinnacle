-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local Log = require("snowcap.log")

---@class snowcap.signal
local signal = {}

---@enum snowcap.signal.HandlerPolicy
signal.HandlerPolicy = {
    ---Keep the handler
    Keep = false,
    ---Discard the handler
    Discard = true,
}

---Stores a callback.
---@class snowcap.signal.SignalCallback
---@field id integer
---@field callback fun(...): snowcap.signal.HandlerPolicy?

---Handle to a signal callback.
---@class snowcap.signal.SignalHandle
---@field private callback? snowcap.signal.SignalCallback
---@field private entry? snowcap.signal.SignalEntry
local SignalHandle = {}

---Disconnects the callback managed by this handle.
function SignalHandle:disconnect()
    if self.entry and self.callback then
        self.entry:remove_callback(self.callback)
    end
end

---Converts a `SignalHandle` into a printable string.
---
---@param handle snowcap.signal.SignalHandle
---
---@return string
function SignalHandle.tostring(handle)
    if handle.entry then
        return "SignalHandle{" .. handle.entry.signal .. "#" .. tostring(handle.callback.id) .. "}"
    else
        return "SignalHandle{StaleHandle}"
    end
end

---@private
---@class snowcap.signal.SignalEntry
---@field id integer
---@field signal string Name of the signal in this entry
---@field signals snowcap.signal.SignalCallback[]
local SignalEntry = {}

---Creates a new SignalEntry.
---
---@private
---@param signal string Signal name.
---@nodiscard
---@return snowcap.signal.SignalEntry
function SignalEntry.new(signal)
    local entry = {
        id = 0,
        signal = signal,
        signals = {},
    }

    setmetatable(entry, { __index = SignalEntry })
    return entry
end

---Gets a valid id for a callback.
---
---@private
---@nodiscard
---@return integer
function SignalEntry:next_id()
    local newid = self.id
    self.id = self.id + 1

    return newid
end

---Adds a new callback for this entry.
---
---@param callback fun(...)
---@nodiscard
---@return snowcap.signal.SignalHandle
function SignalEntry:add_callback(callback)
    ---@type snowcap.signal.SignalCallback
    local signal = {
        id = self:next_id(),
        callback = callback,
    }

    table.insert(self.signals, signal)

    local handle = setmetatable({
        entry = self,
        callback = signal,
    }, { __index = SignalHandle, __tostring = SignalHandle.tostring, __mode = "kv" })

    return handle
end

---Removes a callback from this entry.
---
---@param signal_cb snowcap.signal.SignalCallback
function SignalEntry:remove_callback(signal_cb)
    local idx = nil

    for k, callback in pairs(self.signals) do
        if callback == signal_cb then
            idx = k
            break
        end
    end

    if idx ~= nil then
        table.remove(self.signals, idx)
    end
end

---Emits the message corresponding to this entry.
---
---@param ... any Parameters to pass to the callbacks
function SignalEntry:emit(...)
    local to_remove = {}

    for _, callback in pairs(self.signals) do
        local ok, ret = pcall(callback.callback, ...)

        if ok and ret == signal.HandlerPolicy.Discard then
            to_remove = callback
        elseif not ok then
            Log.error("While handling '" .. self.signal .. "': " .. ret)
        end
    end

    for _, callback in pairs(to_remove) do
        self:remove_callback(callback)
    end
end

---Removes all callbacks from this entry.
function SignalEntry:clear()
    self.signals = {}
end

---Signal emitter.
---
---@class snowcap.signal.Signaler
---@field private entries table<string, snowcap.signal.SignalEntry>
local Signaler = {}
Signaler.__index = Signaler

---Gets the `SignalEntry` associated with a signal, or returns a new entry.
---
---@private
---@param signal string Signal we want the entry to
---@nodiscard
---@return snowcap.signal.SignalEntry
function Signaler:get_or_default(signal)
    self.entries[signal] = self.entries[signal] or SignalEntry.new(signal)

    return self.entries[signal]
end

---Gets the `SignalEntry` associated with a signal.
---
---@private
---@param name string Signal we want the entry to
---@return snowcap.signal.SignalEntry?
function Signaler:get(name)
    return self.entries[name]
end

---Emits a signal.
---
---@param name string Signal to emit
---@param ... any Signal callback parameters
function Signaler:emit(name, ...)
    local entry = self:get(name)

    if not entry then
        return
    end

    entry:emit(...)
end

---Connects a callback to a specific signal.
---
---@param name string Signal to connect to
---@param callback fun(...): boolean? Callback to register
---@return snowcap.signal.SignalHandle
function Signaler:connect(name, callback)
    local entry = self:get_or_default(name)

    return entry:add_callback(callback)
end

---Disconnects a callback managed by a handle.
---
---@param handle snowcap.signal.SignalHandle Handle to the signal we want to disconnect
function Signaler:disconnect(handle)
    ---@diagnostic disable: invisible
    if handle.entry then
        local entry = self:get(handle.entry.signal)

        if entry then
            entry:remove_callback(handle.callback)
        else
            Log.error(tostring(handle) .. " wasn't meant for this Signaler")
        end
    end
end

---Disconnects all callbacks from this table.
function Signaler:disconnect_all()
    for _, entry in pairs(self.entries) do
        entry:clear()
    end

    self.entries = {}
end

---Constructs a new Signaler.
---
---@return snowcap.signal.Signaler
function Signaler.new()
    ---@type snowcap.signal.Signaler
    local self = {
        entries = {},
    }

    setmetatable(self, Signaler)
    return self
end

signal.Signaler = Signaler

return signal
