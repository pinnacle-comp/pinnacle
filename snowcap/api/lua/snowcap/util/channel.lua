local cprom = require("cqueues.promise")

---@class cqueues.promise
---@field get fun(self, timeout?: number): ...
---@field set fun(self, ok: boolean, ...)

---Simple channels implementation.
---@class snowcap.util.channel
local channel = {}

---Sending end of a channel.
---
---@class Sender<T>
---@field _promise cqueues.promise
local Sender = {}

---Receiving end of a channel.
---
---@class Receiver<T>
---@field _promise cqueues.promise
local Receiver = {}

---Send a message on the channel.
---
---@generic T
---@param msg T
function Sender:send(msg)
    if self._promise == nil then
        return false, "Already closed"
    end

    local promise = self._promise
    local next = cprom.new()

    promise:set(true, msg, next)
    self._promise = next
    return true
end

function Sender:close()
    self._promise:set(true, nil, nil)
    self._promise = nil
end

---Wait on this channel for a new message.
---
---@generic T
---@return T?
---@return string?
function Receiver:recv()
    if self._promise then
        local msg, next = self._promise:get()

        self._promise = next
        return msg
    else
        return nil, "Sender closed."
    end
end

---Simple single producer simple receiver channel.
---
---This channel is unbounded.
function channel.spsc()
    local promise = cprom.new()
    local sender = setmetatable({
        _promise = promise
    }, { __index = Sender })
    local receiver = setmetatable({
        _promise = promise,
    }, { __index = Receiver })

    return sender, receiver
end

return channel
