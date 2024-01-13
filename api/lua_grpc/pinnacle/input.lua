---The protobuf absolute path prefix
local prefix = "pinnacle.input." .. require("pinnacle").version .. "."
local service = prefix .. "InputService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) InputServiceMethod
local rpc_types = {
    SetKeybind = {
        response_type = "SetKeybindResponse",
    },
    SetMousebind = {
        response_type = "SetMousebindResponse",
    },
    SetXkbConfig = {},
    SetRepeatRate = {},
    SetLibinputSetting = {},
}

---Build GrpcRequestParams
---@param method InputServiceMethod
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

---@enum Modifier
local modifier = {
    SHIFT = 1,
    CTRL = 2,
    ALT = 3,
    SUPER = 4,
}

---@enum MouseButton
local mouse_button = {
    --- Left
    [1] = 0x110,
    --- Right
    [2] = 0x111,
    --- Middle
    [3] = 0x112,
    --- Side
    [4] = 0x113,
    --- Extra
    [5] = 0x114,
    --- Forward
    [6] = 0x115,
    --- Back
    [7] = 0x116,
    left = 0x110,
    right = 0x111,
    middle = 0x112,
    side = 0x113,
    extra = 0x114,
    forward = 0x115,
    back = 0x116,
}

---@enum MouseEdge
local mouse_edge = {
    PRESS = 1,
    RELEASE = 2,
}

---@class InputModule
local input = {}

---@class Input
---@field private config_client Client
local Input = {
    mod = modifier,
    btn = mouse_button,
    edge = mouse_edge,
}

---@param mods Modifier[] TODO: accept strings of mods
---@param key integer | string
---@param action fun()
function Input:set_keybind(mods, key, action)
    local raw_code = nil
    local xkb_name = nil

    if type(key) == "number" then
        raw_code = key
    elseif type(key) == "string" then
        xkb_name = key
    end

    self.config_client:server_streaming_request(
        build_grpc_request_params("SetKeybind", {
            modifiers = mods,
            raw_code = raw_code,
            xkb_name = xkb_name,
        }),
        action
    )
end

---Set a mousebind.
---
---@param mods Modifier[]
---@param button MouseButton
---@param edge MouseEdge|"press"|"release"
---@param action fun()
function Input:set_mousebind(mods, button, edge, action)
    local edge = edge
    if edge == "press" then
        edge = mouse_edge.PRESS
    elseif edge == "release" then
        edge = mouse_edge.RELEASE
    end

    self.config_client:server_streaming_request(
        build_grpc_request_params("SetMousebind", {
            modifiers = mods,
            button = button,
            edge = edge,
        }),
        action
    )
end

---@class XkbConfig
---@field rules string?
---@field model string?
---@field layout string?
---@field variant string?
---@field options string?

---Set the xkbconfig for your keyboard.
---
---@param xkb_config XkbConfig
function Input:set_xkb_config(xkb_config)
    self.config_client:unary_request(build_grpc_request_params("SetXkbConfig", xkb_config))
end

---Set the keyboard's repeat rate and delay.
---
---@param rate integer The time between repeats, in milliseconds
---@param delay integer The duration a key needs to be held down before repeating starts, in milliseconds
function Input:set_repeat_rate(rate, delay)
    self.config_client:unary_request(build_grpc_request_params("SetRepeatRate", {
        rate = rate,
        delay = delay,
    }))
end

function input.new(config_client)
    ---@type Input
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Input })
    return self
end

return input
