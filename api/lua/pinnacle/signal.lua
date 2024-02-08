---The protobuf absolute path prefix
local prefix = "pinnacle.signal." .. require("pinnacle").version .. "."
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

---@enum SignalEnum
local signal_enum = {
    [0] = "SIGNAL_UNSPECIFIED",
    [1] = "SIGNAL_INPUT_POINTER_BUTTON",
    [2] = "SIGNAL_INPUT_POINTER_MOTION",
    [3] = "SIGNAL_INPUT_POINTER_AXIS",
    [4] = "SIGNAL_INPUT_KEYBOARD",

    [5] = "SIGNAL_WINDOW_POINTER_BUTTON",
    [6] = "SIGNAL_WINDOW_POINTER_MOTION",
    [7] = "SIGNAL_WINDOW_POINTER_AXIS",
    [8] = "SIGNAL_WINDOW_POINTER_ENTER",
    [9] = "SIGNAL_WINDOW_POINTER_LEAVE",
    [10] = "SIGNAL_WINDOW_KEYBOARD",
    [11] = "SIGNAL_WINDOW_OPEN",
    [12] = "SIGNAL_WINDOW_CLOSE",
    [13] = "SIGNAL_WINDOW_FULLSCREEN",
    [14] = "SIGNAL_WINDOW_MAXIMIZE",
    [15] = "SIGNAL_WINDOW_FLOATING",
    [16] = "SIGNAL_WINDOW_MOVE",
    [17] = "SIGNAL_WINDOW_RESIZE",

    [18] = "SIGNAL_OUTPUT_CONNECT",
    [19] = "SIGNAL_OUTPUT_DISCONNECT",
    [20] = "SIGNAL_OUTPUT_MOVE",

    [21] = "SIGNAL_TAG_ADD",
    [22] = "SIGNAL_TAG_REMOVE",
    [23] = "SIGNAL_TAG_ACTIVE",
    [24] = "SIGNAL_TAG_WINDOW_TAGGED",
    [25] = "SIGNAL_TAG_WINDOW_UNTAGGED",
}

---@enum SignalValueEnum
local signal_value_enum = {
    SIGNAL_UNSPECIFIED = 0,
    SIGNAL_INPUT_POINTER_BUTTON = 1,
    SIGNAL_INPUT_POINTER_MOTION = 2,
    SIGNAL_INPUT_POINTER_AXIS = 3,
    SIGNAL_INPUT_KEYBOARD = 4,

    SIGNAL_WINDOW_POINTER_BUTTON = 5,
    SIGNAL_WINDOW_POINTER_MOTION = 6,
    SIGNAL_WINDOW_POINTER_AXIS = 7,
    SIGNAL_WINDOW_POINTER_ENTER = 8,
    SIGNAL_WINDOW_POINTER_LEAVE = 9,
    SIGNAL_WINDOW_KEYBOARD = 10,
    SIGNAL_WINDOW_OPEN = 11,
    SIGNAL_WINDOW_CLOSE = 12,
    SIGNAL_WINDOW_FULLSCREEN = 13,
    SIGNAL_WINDOW_MAXIMIZE = 14,
    SIGNAL_WINDOW_FLOATING = 15,
    SIGNAL_WINDOW_MOVE = 16,
    SIGNAL_WINDOW_RESIZE = 17,

    SIGNAL_OUTPUT_CONNECT = 18,
    SIGNAL_OUTPUT_DISCONNECT = 19,
    SIGNAL_OUTPUT_MOVE = 20,

    SIGNAL_TAG_ADD = 21,
    SIGNAL_TAG_REMOVE = 22,
    SIGNAL_TAG_ACTIVE = 23,
    SIGNAL_TAG_WINDOW_TAGGED = 24,
    SIGNAL_TAG_WINDOW_UNTAGGED = 25,
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

---@class SignalModule
local signal = {}

---@class Signal
---@field private config_client Client
local Signal = {}

---@param sig SignalEnum
function Signal:connect_signal(sig)
    self.config_client:unary_request(build_grpc_request_params("ConnectSignal", {
        signal = signal_value_enum[sig],
    }))
end

---@param sig SignalEnum
function Signal:disconnect_signal(sig)
    self.config_client:unary_request(build_grpc_request_params("DisconnectSignal", {
        signal = signal_value_enum[sig],
    }))
end

function Signal:listen()
    self.config_client:server_streaming_request(build_grpc_request_params("Listen", {}), function(response)
        print(require("inspect")(response))
        for signal_name, signal_values in pairs(response) do
            local pin = require("pinnacle")
            if pin.callbacks[signal_name] then
                pin.callbacks[signal_name](signal_values)
            end
        end
    end)
end

function signal.new(config_client)
    ---@type Signal
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Signal })
    return self
end

return signal
