-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local signal_service = require("pinnacle.grpc.defs").pinnacle.signal.v1.SignalService

local stream_control = require("pinnacle.grpc.defs").pinnacle.signal.v1.StreamControl

local signals = {
    OutputConnect = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputDisconnect = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputSetup = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputResize = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle, logical_width: integer, logical_height: integer) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputMove = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle, x: integer, y: integer) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputPointerEnter = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputPointerLeave = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputFocused = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(output: pinnacle.output.OutputHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerEnter = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(window: pinnacle.window.WindowHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerLeave = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(window: pinnacle.window.WindowHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowFocused = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(window: pinnacle.window.WindowHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowTitleChanged = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(window: pinnacle.window.WindowHandle, title: string) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    TagActive = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(tag: pinnacle.tag.TagHandle, active: boolean) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    InputDeviceAdded = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type { callback_id: integer, callback: fun(device: pinnacle.input.libinput.DeviceHandle) }[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
}

---Call a signal callback in protected mode
---
---If the signal fails, the error is logged.
---The handle parameter is expected to be the first parameter of the callback, and to be convertible
---to a string via tostring(). If it's not the case, setting this parameter to nil will not pass it
---to the callback, and will not uses in in the error string generation.
---
---@generic T
---@param signal_name string
---@param callback function
---@param handle T|nil Either a Handle object that'll be passed as callback first arg, or nil
local function protected_callback(signal_name, callback, handle, ...)
    local success, err

    if handle then
        success, err = pcall(callback, handle, ...)
    else
        success, err = pcall(callback, ...)
    end

    if not success then
        local errstr = "While handling '" .. signal_name .. "'"
        if handle then
            errstr = errstr .. " for " .. tostring(handle)
        end
        errstr = errstr .. ": " .. tostring(err)

        log.error(errstr)
    end
end

-- NOTE: We need to copy callbacks into a new table because callbacks are able to disconnect signals
-- while we iterate through them, which is a no-no

signals.OutputConnect.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputConnect.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputConnect", callback.callback, handle)
    end
end

signals.OutputDisconnect.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputDisconnect.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputDisconnect", callback.callback, handle)
    end
end

signals.OutputSetup.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputSetup.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputSetup", callback.callback, handle)
    end
end

signals.OutputResize.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputResize.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback(
            "OutputResize",
            callback.callback,
            handle,
            response.logical_width,
            response.logical_height
        )
    end
end

signals.OutputMove.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputMove.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputMove", callback.callback, handle, response.x, response.y)
    end
end

signals.OutputPointerEnter.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputPointerEnter.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputPointerEnter", callback.callback, handle)
    end
end

signals.OutputPointerLeave.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputPointerLeave.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputPointerLeave", callback.callback, handle)
    end
end

signals.OutputFocused.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    local callbacks = require("pinnacle.util").deep_copy(signals.OutputFocused.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("OutputFocused", callback.callback, handle)
    end
end

signals.WindowPointerEnter.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)
    local callbacks = require("pinnacle.util").deep_copy(signals.WindowPointerEnter.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("WindowPointerEnter", callback.callback, window_handle)
    end
end

signals.WindowPointerLeave.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)
    local callbacks = require("pinnacle.util").deep_copy(signals.WindowPointerLeave.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("WindowPointerLeave", callback.callback, window_handle)
    end
end

signals.WindowFocused.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)
    local callbacks = require("pinnacle.util").deep_copy(signals.WindowFocused.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("WindowFocused", callback.callback, window_handle)
    end
end

signals.WindowTitleChanged.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)
    local callbacks = require("pinnacle.util").deep_copy(signals.WindowTitleChanged.callbacks)
    local title = response.title or ""

    for _, callback in ipairs(callbacks) do
        protected_callback("WindowTitleChanged", callback.callback, window_handle, title)
    end
end

signals.TagActive.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local tag_handle = require("pinnacle.tag").handle.new(response.tag_id)
    local callbacks = require("pinnacle.util").deep_copy(signals.TagActive.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("TagActive", callback.callback, tag_handle, response.active)
    end
end

signals.InputDeviceAdded.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local device_handle = require("pinnacle.input.libinput").new_device(response.device_sysname)
    local callbacks = require("pinnacle.util").deep_copy(signals.InputDeviceAdded.callbacks)

    for _, callback in ipairs(callbacks) do
        protected_callback("InputDeviceAdded", callback.callback, device_handle)
    end
end

-----------------------------------------------------------------------------

---@class pinnacle.signal.SignalHandleModule
---@lcat nodoc
local signal_handle = {}

---A handle to a connected signal that can be used to disconnect the provided callback.
---
---@class pinnacle.signal.SignalHandle
---@lcat nodoc
---@field private signal string
---@lcat nodoc
---@field private callback_id integer The ID for the callback you connected
local SignalHandle = {}

---@class pinnacle.signal.SignalHandlesModule
---@lcat nodoc
local signal_handles = {}

---A collection of `SignalHandle`s retreived through a `connect_signal` function.
---@class pinnacle.signal.SignalHandles
local SignalHandles = {}

---@class pinnacle.signal.Signal
---@field private handle pinnacle.signal.SignalHandleModule
---@field private handles pinnacle.signal.SignalHandlesModule
---@lcat nodoc
local signal = {}
signal.handle = signal_handle
signal.handles = signal_handles

---Disconnect the provided callback from this signal.
function SignalHandle:disconnect()
    local cb_index = nil
    for i, cb in ipairs(signals[self.signal].callbacks) do
        if cb.callback_id == self.callback_id then
            cb_index = i
            break
        end
    end

    if cb_index then
        table.remove(signals[self.signal].callbacks, cb_index)
    end

    if #signals[self.signal].callbacks == 0 then
        signal.disconnect(self.signal)
    end
end

---@param request string
---@param callback_id integer
---@return pinnacle.signal.SignalHandle
function signal_handle.new(request, callback_id)
    ---@type pinnacle.signal.SignalHandle
    local self = {
        signal = request,
        callback_id = callback_id,
    }
    setmetatable(self, { __index = SignalHandle })
    return self
end

---Disconnects the callbacks from all the signal connections that are stored in this handle collection.
---
---@param self table<string, pinnacle.signal.SignalHandle>
function SignalHandles:disconnect_all()
    for _, sig in pairs(self) do
        sig:disconnect()
    end
end

---@return pinnacle.signal.SignalHandles
function signal_handles.new()
    ---@type pinnacle.signal.SignalHandles
    local self = {}
    setmetatable(self, { __index = SignalHandles })
    return self
end

local callback_ids = 0

---@param request string
---@param callback function
---@return pinnacle.signal.SignalHandle
---@lcat nodoc
function signal.add_callback(request, callback)
    if #signals[request].callbacks == 0 then
        signal.connect(request, signals[request].on_response)
    end

    local next_id = callback_ids
    callback_ids = callback_ids + 1

    table.insert(signals[request].callbacks, {
        callback_id = next_id,
        callback = callback,
    })

    return signal_handle.new(request, next_id)
end

---@param request string
---@param callback fun(response: table)
---@lcat nodoc
function signal.connect(request, callback)
    local stream = client:bidirectional_streaming_request(
        signal_service[request],
        function(response)
            callback(response)

            if signals[request].sender then
                local chunk = require("pinnacle.grpc.protobuf").encode(
                    "pinnacle.signal.v1." .. request .. "Request",
                    {
                        control = stream_control.STREAM_CONTROL_READY,
                    }
                )

                local success, err =
                    pcall(signals[request].sender.write_chunk, signals[request].sender, chunk)

                if not success then
                    print("error sending to stream:", err)
                    os.exit(1)
                end
            end
        end
    )

    signals[request].sender = stream

    local chunk =
        require("pinnacle.grpc.protobuf").encode("pinnacle.signal.v1." .. request .. "Request", {
            control = stream_control.STREAM_CONTROL_READY,
        })

    local success, err = pcall(signals[request].sender.write_chunk, signals[request].sender, chunk)
    if not success then
        print("error sending to stream:", err)
        os.exit(1)
    end
end

---This should only be called when call callbacks for the signal are removed
---@param request string
---@lcat nodoc
function signal.disconnect(request)
    if signals[request].sender then
        local chunk = require("pinnacle.grpc.protobuf").encode(
            "pinnacle.signal.v1." .. request .. "Request",
            {
                control = stream_control.STREAM_CONTROL_DISCONNECT,
            }
        )

        local success, err =
            pcall(signals[request].sender.write_chunk, signals[request].sender, chunk)
        if not success then
            print("error sending to stream:", err)
            os.exit(1)
        end

        signals[request].sender:shutdown()
        signals[request].sender = nil
    end
end

return signal
