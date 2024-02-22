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

local signals = {
    output_connect = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
        callbacks = {},
    },
    layout = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(windows: WindowHandle[], tag: TagHandle))[]
        callbacks = {},
    },
    window_pointer_enter = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
        callbacks = {},
    },
    window_pointer_leave = {
        ---@type H2Stream?
        sender = nil,
        ---@type (fun(output: OutputHandle))[]
        callbacks = {},
    },
}

---@class Signal
local signal = {}

---@param fn fun(windows: WindowHandle[], tag: TagHandle)
function signal.layout_add(fn)
    if #signals.layout.callbacks == 0 then
        signal.layout_connect()
    end

    table.insert(signals.layout.callbacks, fn)
end

function signal.layout_dc()
    signal.layout_disconnect()
end

function signal.output_connect_connect()
    local stream = client.bidirectional_streaming_request(
        build_grpc_request_params("OutputConnect", {
            control = stream_control.READY,
        }),
        function(response)
            ---@diagnostic disable-next-line: invisible
            local handle = require("pinnacle.output").handle.new(response.output_name)
            for _, callback in ipairs(signals.output_connect.callbacks) do
                callback(handle)
            end

            local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. "OutputConnectRequest", {
                control = stream_control.READY,
            })

            if signals.layout.sender then
                signals.layout.sender:write_chunk(chunk)
            end
        end
    )

    signals.output_connect.sender = stream
end

function signal.output_connect_disconnect()
    if signals.output_connect.sender then
        local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. "OutputConnectRequest", {
            control = stream_control.DISCONNECT,
        })

        signals.output_connect.sender:write_chunk(chunk)
        signals.output_connect.sender = nil
    end
end

function signal.layout_connect()
    local stream = client.bidirectional_streaming_request(
        build_grpc_request_params("Layout", {
            control = stream_control.READY,
        }),
        function(response)
            ---@diagnostic disable-next-line: invisible
            local window_handles = require("pinnacle.window").handle.new_from_table(response.window_ids or {})
            ---@diagnostic disable-next-line: invisible
            local tag_handle = require("pinnacle.tag").handle.new(response.tag_id)

            for _, callback in ipairs(signals.layout.callbacks) do
                print("calling layout callback")
                callback(window_handles, tag_handle)
            end

            print("creating control request")
            local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. "LayoutRequest", {
                control = stream_control.READY,
            })

            if signals.layout.sender then
                local success, err = pcall(signals.layout.sender.write_chunk, signals.layout.sender, chunk)
                if not success then
                    print("error sending to stream:", err)
                    os.exit(1)
                end
            end
        end
    )

    signals.layout.sender = stream
end

function signal.layout_disconnect()
    if signals.layout.sender then
        local chunk = require("pinnacle.grpc.protobuf").encode(prefix .. "LayoutRequest", {
            control = stream_control.DISCONNECT,
        })

        signals.layout.sender:write_chunk(chunk)
        signals.layout.sender = nil
    end
    signals.layout.callbacks = {}
end

return signal
