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

-- This is an @enum and not an @alias because with an @alias the completion replaces tables with a string,
-- which is annoying

---@enum (key) Modifier
local modifier_values = {
    shift = 1,
    ctrl = 2,
    alt = 3,
    super = 4,
}

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
    btn_left = 0x110,
    btn_right = 0x111,
    btn_middle = 0x112,
    btn_side = 0x113,
    btn_extra = 0x114,
    btn_forward = 0x115,
    btn_back = 0x116,
}
-- This alias is because I can't get @enum completion to work
---@alias MouseButton
---| 1 Left
---| 2 Right
---| 3 Middle
---| 4 Side
---| 5 Extra
---| 6 Forward
---| 7 Back,
---| "btn_left"
---| "btn_right"
---| "btn_middle"
---| "btn_side"
---| "btn_extra"
---| "btn_forward"
---| "btn_back"

local mouse_edge_values = {
    press = 1,
    release = 2,
}
---@alias MouseEdge
---| "press" Trigger on mouse button press
---| "release" Trigger on mouse button release

---@nodoc
---@class InputModule
---@field private btn table
local input = {}
input.btn = mouse_button_values

---Input management.
---
---This module provides utilities to set key- and mousebinds as well as change keyboard settings.
---@class Input
---@field private config_client Client
local Input = {
    key = require("pinnacle.input.keys"),
}

---Set a keybind. If called with an already existing keybind, it gets replaced.
---
---You must provide three arguments:
---
--- - `mods`: An array of `Modifier`s. If you don't want any, provide an empty table.
--- - `key`: The key that will trigger `action`. You can provide three types of key:
---     - Something from the `Key` table in `Input.key`, which lists every xkbcommon key. The naming pattern is the xkbcommon key without the `KEY_` prefix, unless that would make it start with a number or the reserved lua keyword `function`, in which case the `KEY_` prefix is included.
---     - A single character representing your key. This can be something like "g", "$", "~", "1", and so on.
---     - A string of the key's name. This is the name of the xkbcommon key without the `KEY_` prefix.
--- - `action`: The function that will be run when the keybind is pressed.
---
---It is important to note that `"a"` is different than `"A"`. Similarly, `key.a` is different than `key.A`.
---Usually, it's best to use the non-modified key to prevent confusion and unintended behavior.
---
---```lua
---Input:keybind({ "shift" }, "a", function() end) -- This is preferred
---Input:keybind({ "shift" }, "A", function() end) -- over this
---
--- -- This keybind will only work with capslock on.
---Input:keybind({}, "A", function() end)
---
--- -- This keybind won't work at all because to get `@` you need to hold shift,
--- -- which this keybind doesn't accept.
---Input:keybind({ "ctrl" }, "@", function() end)
---```
---
---### Example
---```lua
--- -- Set `super + Return` to open Alacritty
---Input:keybind({ "super" }, Input.key.Return, function()
---    Process:spawn("alacritty")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param key Key | string The key used to trigger the bind
---@param action fun() The function to run when the bind is triggered
function Input:keybind(mods, key, action)
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

---Set a mousebind. If called with an already existing mousebind, it gets replaced.
---
---You must specify whether the keybind happens on button press or button release.
---
---### Example
---```lua
--- -- Set `super + left mouse button` to move a window on press
---Input:mousebind({ "super" }, "btn_left", "press", function()
---    Window:begin_move("btn_left")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param button MouseButton The mouse button used to trigger the bind
---@param edge MouseEdge "press" or "release" to trigger on button press or release
---@param action fun() The function to run when the bind is triggered
function Input:mousebind(mods, button, edge, action)
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
---Fields not present will be set to their default values.
---
---Read `xkeyboard-config(7)` for more information.
---
---### Example
---```lua
---Input:set_xkb_config({
---    layout = "us,fr,ge",
---    options = "ctrl:swapcaps,caps:shift"
---})
---```
---
---@param xkb_config XkbConfig The new xkbconfig
function Input:set_xkb_config(xkb_config)
    self.config_client:unary_request(build_grpc_request_params("SetXkbConfig", xkb_config))
end

---Set the keyboard's repeat rate and delay.
---
---### Example
---```lua
---Input:set_repeat_rate(100, 1000) -- Key must be held down for 1 second, then repeats 10 times per second.
---```
---
---@param rate integer The time between repeats in milliseconds
---@param delay integer The duration a key needs to be held down before repeating starts in milliseconds
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
