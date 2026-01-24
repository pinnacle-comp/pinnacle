-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local signal = require("snowcap.signal")

local widget_id = 0

local function next_id()
    local id = widget_id
    widget_id = widget_id + 1
    return id
end

---The base class for all widget programs.
---
---This provides common functionality including
---unique identifiers and signals.
---
---@class snowcap.widget.base.Base
---@field private signaler snowcap.signal.Signaler
---@field private widget_id integer
local Base = {}
Base.__index = Base

---Connects a callback to a specific signal.
---
---@param name string The name of the signal you're connecting to.
---@return snowcap.signal.SignalHandle
function Base:connect(name, callback)
    return self.signaler:connect(name, callback)
end

---Emits a signal.
---
---@param name string Signal to emit
---@param ... any Parameter to sent to the callbacks
function Base:emit(name, ...)
    self.signaler:emit(name, ...)
end

---Disconnects a given callback.
---
---@param handle snowcap.signal.SignalHandle Handle to the callback to disconnect.
function Base:disconnect(handle)
    self.signaler:disconnect(handle)
end

---Disconnects all signal handlers.
function Base:disconnect_all()
    self.signaler:disconnect_all()
end

---Gets the widget's unique id.
---
---@return integer
function Base:id()
    return self.widget_id
end

---Creates a new widget base.
---
---@return snowcap.widget.base.Base
function Base.new()
    ---@type snowcap.widget.base.Base
    local self = {
        widget_id = next_id(),
        signaler = signal.Signaler.new(),
    }

    setmetatable(self, Base)
    return self
end

---Widget base module.
---
---@class snowcap.widget.base
local base = {
    Base = Base,
}

return base
