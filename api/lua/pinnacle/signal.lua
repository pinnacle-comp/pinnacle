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
    ConnectSignal = {},
    DisconnectSignal = {},
    Listen = {
        response_type = "ListenResponse",
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

---@enum SignalEnum
local signal_enum = {
    [0] = "SIGNAL_UNSPECIFIED",
    [1] = "SIGNAL_INPUT_POINTER_BUTTON",
    [2] = "SIGNAL_INPUT_POINTER_MOTION",
    [3] = "SIGNAL_INPUT_POINTER_AXIS",
    [4] = "SIGNAL_INPUT_KEYBOARD",

    [5] = "SIGNAL_WINDOW_POINTER_ENTER",
    [6] = "SIGNAL_WINDOW_POINTER_LEAVE",
    [7] = "SIGNAL_WINDOW_OPEN",
    [8] = "SIGNAL_WINDOW_CLOSE",
    [9] = "SIGNAL_WINDOW_FULLSCREEN",
    [10] = "SIGNAL_WINDOW_MAXIMIZE",
    [11] = "SIGNAL_WINDOW_FLOATING",
    [12] = "SIGNAL_WINDOW_MOVE",
    [13] = "SIGNAL_WINDOW_RESIZE",

    [14] = "SIGNAL_OUTPUT_CONNECT",
    [15] = "SIGNAL_OUTPUT_DISCONNECT",
    [16] = "SIGNAL_OUTPUT_MOVE",

    [17] = "SIGNAL_TAG_ADD",
    [18] = "SIGNAL_TAG_REMOVE",
    [19] = "SIGNAL_TAG_ACTIVE",
    [20] = "SIGNAL_TAG_WINDOW_TAGGED",
    [21] = "SIGNAL_TAG_WINDOW_UNTAGGED",
}

---@enum SignalValueEnum
local signal_value_enum = {
    SIGNAL_UNSPECIFIED = 0,
    SIGNAL_INPUT_POINTER_BUTTON = 1,
    SIGNAL_INPUT_POINTER_MOTION = 2,
    SIGNAL_INPUT_POINTER_AXIS = 3,
    SIGNAL_INPUT_KEYBOARD = 4,

    SIGNAL_WINDOW_POINTER_ENTER = 5,
    SIGNAL_WINDOW_POINTER_LEAVE = 6,
    SIGNAL_WINDOW_OPEN = 7,
    SIGNAL_WINDOW_CLOSE = 8,
    SIGNAL_WINDOW_FULLSCREEN = 9,
    SIGNAL_WINDOW_MAXIMIZE = 10,
    SIGNAL_WINDOW_FLOATING = 11,
    SIGNAL_WINDOW_MOVE = 12,
    SIGNAL_WINDOW_RESIZE = 13,

    SIGNAL_OUTPUT_CONNECT = 14,
    SIGNAL_OUTPUT_DISCONNECT = 15,
    SIGNAL_OUTPUT_MOVE = 16,

    SIGNAL_TAG_ADD = 17,
    SIGNAL_TAG_REMOVE = 18,
    SIGNAL_TAG_ACTIVE = 19,
    SIGNAL_TAG_WINDOW_TAGGED = 20,
    SIGNAL_TAG_WINDOW_UNTAGGED = 21,
}

---@type table<ListenResponseSignalName, SignalEnum>
local listen_response_signal_name_to_signal_enum = {
    input_pointer_button = "SIGNAL_INPUT_POINTER_BUTTON",
    input_pointer_motion = "SIGNAL_INPUT_POINTER_MOTION",
    input_pointer_axis = "SIGNAL_INPUT_POINTER_AXIS",
    input_keyboard = "SIGNAL_INPUT_KEYBOARD",

    window_pointer_enter = "SIGNAL_WINDOW_POINTER_ENTER",
    window_pointer_leave = "SIGNAL_WINDOW_POINTER_LEAVE",
    window_open = "SIGNAL_WINDOW_OPEN",
    window_close = "SIGNAL_WINDOW_CLOSE",
    window_fullscreen = "SIGNAL_WINDOW_FULLSCREEN",
    window_maximize = "SIGNAL_WINDOW_MAXIMIZE",
    window_floating = "SIGNAL_WINDOW_FLOATING",
    window_move = "SIGNAL_WINDOW_MOVE",
    window_resize = "SIGNAL_WINDOW_RESIZE",

    output_connect = "SIGNAL_OUTPUT_CONNECT",
    output_disconnect = "SIGNAL_OUTPUT_DISCONNECT",
    output_move = "SIGNAL_OUTPUT_MOVE",

    tag_add = "SIGNAL_TAG_ADD",
    tag_remove = "SIGNAL_TAG_REMOVE",
    tag_active = "SIGNAL_TAG_ACTIVE",
    tag_window_tagged = "SIGNAL_TAG_WINDOW_TAGGED",
    tag_window_untagged = "SIGNAL_TAG_WINDOW_UNTAGGED",
}

---@class Signal
local signal = {
    ---@enum (key) ListenResponseSignalName
    callbacks = {
        ---@type fun(response: { window_id: integer?, code: integer, state: integer, x: number, y: number })[]
        input_pointer_button = {},
        ---@type fun(response: { window_id: integer?, x: number, y: number, rel_x: number, rel_y: number })[]
        input_pointer_motion = {},
        ---@type fun(response: { window_id: integer?, vertical_value: number, horizontal_value: number, vertical_value_discrete: integer, horizontal_value_discrete: integer })[]
        input_pointer_axis = {},
        ---@type fun(response: { window_id: integer?, code: integer, state: integer, raw_keysyms: integer[], modified_keysyms: integer[] })[]
        input_keyboard = {},

        ---@type fun(response: { window_id: integer })[]
        window_pointer_enter = {},
        ---@type fun(response: { window_id: integer })[]
        window_pointer_leave = {},
        ---@type fun(response: { window_id: integer })[]
        window_open = {},
        ---@type fun(response: { window_id: integer })[]
        window_close = {},
        ---@type fun(response: { window_id: integer, fullscreen: boolean })[]
        window_fullscreen = {},
        ---@type fun(response: { window_id: integer, maximize: boolean })[]
        window_maximize = {},
        ---@type fun(response: { window_id: integer, floating: boolean })[]
        window_floating = {},
        ---@type fun(response: { window_id: integer, x: integer, y: integer })[]
        window_move = {},
        ---@type fun(response: { window_id: integer, width: integer, height: integer })[]
        window_resize = {},

        ---@type fun(response: { output_name: string })[]
        output_connect = {},
        ---@type fun(response: { output_name: string })[]
        output_disconnect = {},
        ---@type fun(response: { output_name: string, x: integer, y: integer })[]
        output_move = {},

        ---@type fun(response: { tag_id: integer })[]
        tag_add = {},
        ---@type fun(response: { tag_id: integer })[]
        tag_remove = {},
        ---@type fun(response: { tag_id: integer, active: boolean })[]
        tag_active = {},
        ---@type fun(response: { tag_id: integer, window_id: integer })[]
        tag_window_tagged = {},
        ---@type fun(response: { tag_id: integer, window_id: integer })[]
        tag_window_untagged = {},
    },
}

---@param signal_name ListenResponseSignalName
---@param callback function
function signal.insert_callback(signal_name, callback)
    if #signal.callbacks[signal_name] == 0 then
        signal.connect_signal(listen_response_signal_name_to_signal_enum[signal_name])
    end

    table.insert(signal.callbacks[signal_name], callback)
end

-- TODO:
-- function signal.remove_callback(signal_name) end

---@param sig SignalEnum
function signal.connect_signal(sig)
    client.unary_request(build_grpc_request_params("ConnectSignal", {
        signal = signal_value_enum[sig],
    }))
end

---@param sig SignalEnum
function signal.disconnect_signal(sig)
    client.unary_request(build_grpc_request_params("DisconnectSignal", {
        signal = signal_value_enum[sig],
    }))
end

function signal.listen()
    client.server_streaming_request(build_grpc_request_params("Listen", {}), function(response)
        -- print(require("inspect")(response))

        ---@type ListenResponseSignalName
        local signal_name = response.signal
        local signal_values = response[signal_name]

        for _, callback in ipairs(signal.callbacks[signal_name]) do
            callback(signal_values)
        end
    end)
end

return signal

-- User facing signal definitions

---@class InputSignal
---@field pointer_button fun(button: MouseButtonName, state: MouseEdge, x: number, y: number)?
---@field pointer_motion fun(x: number, y: number, relative_x: number, relative_y: number)?
---@field pointer_axis fun(vertical: number, horizontal: number, vertical_discrete: integer, horizontal_discrete: integer)?
---@field keyboard fun(key_code: integer, state: MouseEdge, raw_keys: Key[], modified_keys: Key[])?
