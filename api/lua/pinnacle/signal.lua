-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local signal_service = require("pinnacle.grpc.defs").pinnacle.signal.v1.SignalService

local stream_control = require("pinnacle.grpc.defs").pinnacle.signal.v1.StreamControl

local signals = {
    OutputConnect = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputDisconnect = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputResize = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle, logical_width: integer, logical_height: integer))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputMove = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle, x: integer, y: integer))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputPointerEnter = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputPointerLeave = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    OutputFocused = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(output: pinnacle.output.OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerEnter = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(window: pinnacle.window.WindowHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerLeave = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(window: pinnacle.window.WindowHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowFocused = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(window: pinnacle.window.WindowHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    TagActive = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(tag: pinnacle.tag.TagHandle, active: boolean))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    InputDeviceAdded = {
        ---@type grpc_client.h2.Stream?
        sender = nil,
        ---@type (fun(device: pinnacle.input.libinput.DeviceHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
}

signals.OutputConnect.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    for _, callback in ipairs(signals.OutputConnect.callbacks) do
        callback(handle)
    end
end

signals.OutputDisconnect.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    for _, callback in ipairs(signals.OutputDisconnect.callbacks) do
        callback(handle)
    end
end

signals.OutputResize.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    for _, callback in ipairs(signals.OutputResize.callbacks) do
        callback(handle, response.logical_width, response.logical_height)
    end
end

signals.OutputMove.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)
    for _, callback in ipairs(signals.OutputMove.callbacks) do
        callback(handle, response.x, response.y)
    end
end

signals.OutputPointerEnter.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)

    for _, callback in ipairs(signals.OutputPointerEnter.callbacks) do
        callback(handle)
    end
end

signals.OutputPointerLeave.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)

    for _, callback in ipairs(signals.OutputPointerLeave.callbacks) do
        callback(handle)
    end
end

signals.OutputFocused.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local handle = require("pinnacle.output").handle.new(response.output_name)

    for _, callback in ipairs(signals.OutputFocused.callbacks) do
        callback(handle)
    end
end

signals.WindowPointerEnter.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)

    for _, callback in ipairs(signals.WindowPointerEnter.callbacks) do
        callback(window_handle)
    end
end

signals.WindowPointerLeave.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)

    for _, callback in ipairs(signals.WindowPointerLeave.callbacks) do
        callback(window_handle)
    end
end

signals.WindowFocused.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handle = require("pinnacle.window").handle.new(response.window_id)

    for _, callback in ipairs(signals.WindowFocused.callbacks) do
        callback(window_handle)
    end
end

signals.TagActive.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local tag_handle = require("pinnacle.tag").handle.new(response.tag_id)

    for _, callback in ipairs(signals.TagActive.callbacks) do
        callback(tag_handle, response.active)
    end
end

signals.InputDeviceAdded.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local device_handle = require("pinnacle.input.libinput").new_device(response.device_sysname)

    for _, callback in ipairs(signals.InputDeviceAdded.callbacks) do
        callback(device_handle)
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
---@field private callback function The callback you connected
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
        if cb == self.callback then
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

---@return pinnacle.signal.SignalHandle
function signal_handle.new(request, callback)
    ---@type pinnacle.signal.SignalHandle
    local self = {
        signal = request,
        callback = callback,
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

---@param signal_hdls table<string, pinnacle.signal.SignalHandle>
---@return pinnacle.signal.SignalHandles
function signal_handles.new(signal_hdls)
    ---@type pinnacle.signal.SignalHandles
    local self = signal_hdls
    setmetatable(self, { __index = SignalHandles })
    return self
end

---@param request string
---@param callback function
---@lcat nodoc
function signal.add_callback(request, callback)
    if #signals[request].callbacks == 0 then
        signal.connect(request, signals[request].on_response)
    end

    table.insert(signals[request].callbacks, callback)
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
