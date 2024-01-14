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

---@enum (key) Modifier
local modifier_values = {
    shift = 1,
    ctrl = 2,
    alt = 3,
    super = 4,
}

---@enum (key) MouseButton
local mouse_button_values = {
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

---@enum (key) MouseEdge
local mouse_edge_values = {
    press = 1,
    release = 2,
}

---@class InputModule
---@field private btn table
local input = {}
input.btn = mouse_button_values

---@class Input
---@field private config_client Client
local Input = {
    key = require("pinnacle.input.keys"),
}

---@param mods Modifier[]
---@param key Key | string
---@param action fun()
function Input:set_keybind(mods, key, action)
    local raw_code = nil
    local xkb_name = nil

    if type(key) == "number" then
        raw_code = key
    elseif type(key) == "string" then
        xkb_name = key
    end

    local mod_values = {}
    for _, mod in ipairs(mods) do
        table.insert(mod_values, modifier_values[mod])
    end

    self.config_client:server_streaming_request(
        build_grpc_request_params("SetKeybind", {
            modifiers = mod_values,
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
---@param edge MouseEdge
---@param action fun()
function Input:set_mousebind(mods, button, edge, action)
    local edge = mouse_edge_values[edge]

    local mod_values = {}
    for _, mod in ipairs(mods) do
        table.insert(mod_values, modifier_values[mod])
    end

    self.config_client:server_streaming_request(
        build_grpc_request_params("SetMousebind", {
            modifiers = mod_values,
            button = mouse_button_values[button],
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
