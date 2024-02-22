-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")

---The protobuf absolute path prefix
local prefix = "pinnacle.signal." .. client.version .. "."
local service = prefix .. "SignalService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) SignalServiceMethod
local rpc_types = {
    OutputConnect = {
        response_type = "OutputConnectResponse",
    },
    Layout = {
        response_type = "LayoutResponse",
    },
    WindowPointerEnter = {
        response_type = "WindowPointerEnterResponse",
    },
    WindowPointerLeave = {
        response_type = "WindowPointerLeaveResponse",
    },
}

---Build GrpcRequestParams
---@param method SignalServiceMethod
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

local stream_control = {
    UNSPECIFIED = 0,
    READY = 1,
    DISCONNECT = 2,
}

---@type table<SignalServiceMethod, { sender: H2Stream?, callbacks: function[], on_response: fun(response: table) }>
local signals = {
    OutputConnect = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    Layout = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(tag: TagHandle, windows: WindowHandle[]))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerEnter = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
        callbacks = {},
        ---@type fun(response: table)
        on_response = nil,
    },
    WindowPointerLeave = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
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

signals.Layout.on_response = function(response)
    ---@diagnostic disable-next-line: invisible
    local window_handles = require("pinnacle.window").handle.new_from_table(response.window_ids or {})
    ---@diagnostic disable-next-line: invisible
    local tag_handle = require("pinnacle.tag").handle.new(response.tag_id)

    for _, callback in ipairs(signals.Layout.callbacks) do
        callback(tag_handle, window_handles)
    end
end

-----------------------------------------------------------------------------

---@class SignalHandleModule
local signal_handle = {}

---@class SignalHandle
---@field signal SignalServiceMethod
---@field callback function The callback you connected
local SignalHandle = {}

---@class SignalHandlesModule
local signal_handles = {}

---@class SignalHandles
local SignalHandles = {}

---@class Signal
---@field private handle SignalHandleModule
---@field private handles SignalHandlesModule
local signal = {}
signal.handle = signal_handle
signal.handles = signal_handles

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

---@return SignalHandle
function signal_handle.new(request, callback)
    ---@type SignalHandle
    local self = {
        signal = request,
        callback = callback,
    }
    setmetatable(self, { __index = SignalHandle })
    return self
end

---@param self table<string, SignalHandle>
function SignalHandles:disconnect_all()
    for _, sig in pairs(self) do
        sig:disconnect()
    end
end

---@param signal_hdls table<string, SignalHandle>
---@return SignalHandles
function signal_handles.new(signal_hdls)
    ---@type SignalHandles
    local self = signal_hdls
    setmetatable(self, { __index = SignalHandles })
    return self
end

---@param request SignalServiceMethod
---@param callback function
function signal.add_callback(request, callback)
    if #signals[request].callbacks == 0 then
        signal.connect(request, signals[request].on_response)
    end

    table.insert(signals[request].callbacks, callback)
end

---@param request SignalServiceMethod
---@param callback fun(response: table)
function signal.connect(request, callback)
    local stream = client.bidirectional_streaming_request(
        build_grpc_request_params("Layout", {
            control = stream_control.READY,
        }),
        function(response)
            callback(response)

            if signals[request].sender then
                local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. request .. "Request", {
                    control = stream_control.READY,
                })

                local success, err = pcall(signals[request].sender.write_chunk, signals[request].sender, chunk)

                if not success then
                    print("error sending to stream:", err)
                    os.exit(1)
                end
            end
        end
    )

    signals[request].sender = stream
end

---This should only be called when call callbacks for the signal are removed
---@param request SignalServiceMethod
function signal.disconnect(request)
    if signals[request].sender then
        local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. request .. "Request", {
            control = stream_control.DISCONNECT,
        })

        local success, err = pcall(signals[request].sender.write_chunk, signals[request].sender, chunk)
        if not success then
            print("error sending to stream:", err)
            os.exit(1)
        end

        signals[request].sender:shutdown()
        signals[request].sender = nil
    end
end

return signal
