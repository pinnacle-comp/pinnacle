-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local defs = require("pinnacle.grpc.defs")
local input_v0alpha1 = defs.pinnacle.input.v0alpha1
local input_service = defs.pinnacle.input.v0alpha1.InputService

---@enum (key) Modifier
local modifier_values = {
    shift = input_v0alpha1.Modifier.MODIFIER_SHIFT,
    ctrl = input_v0alpha1.Modifier.MODIFIER_CTRL,
    alt = input_v0alpha1.Modifier.MODIFIER_ALT,
    super = input_v0alpha1.Modifier.MODIFIER_SUPER,
}

require("pinnacle.util").make_bijective(modifier_values)

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
    btn_left = 0x110,
    btn_right = 0x111,
    btn_middle = 0x112,
    btn_side = 0x113,
    btn_extra = 0x114,
    btn_forward = 0x115,
    btn_back = 0x116,
}

---@enum (key) MouseEdge
local mouse_edge_values = {
    press = input_v0alpha1.SetMousebindRequest.MouseEdge.MOUSE_EDGE_PRESS,
    release = input_v0alpha1.SetMousebindRequest.MouseEdge.MOUSE_EDGE_RELEASE,
}

---Input management.
---
---This module provides utilities to set key- and mousebinds as well as change keyboard settings.
---@class Input
---@field private mouse_button_values table
local input = {
    key = require("pinnacle.input.keys"),
}
input.mouse_button_values = mouse_button_values

---@class KeybindInfo
---@field group string? The group to place this keybind in. Used for the keybind list.
---@field description string? The description of this keybind. Used for the keybind list.

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
---Input.keybind({ "shift" }, "a", function() end) -- This is preferred
---Input.keybind({ "shift" }, "A", function() end) -- over this
---
--- -- This keybind will only work with capslock on.
---Input.keybind({}, "A", function() end)
---
--- -- This keybind won't work at all because to get `@` you need to hold shift,
--- -- which this keybind doesn't accept.
---Input.keybind({ "ctrl" }, "@", function() end)
---```
---
---### Example
---```lua
--- -- Set `super + Return` to open Alacritty
---Input.keybind({ "super" }, Input.key.Return, function()
---    Process.spawn("alacritty")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param key Key | string The key used to trigger the bind
---@param action fun() The function to run when the bind is triggered
---@param keybind_info KeybindInfo?
function input.keybind(mods, key, action, keybind_info)
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

    client:server_streaming_request(input_service.SetKeybind, {
        modifiers = mod_values,
        raw_code = raw_code,
        xkb_name = xkb_name,
        group = keybind_info and keybind_info.group,
        description = keybind_info and keybind_info.description,
    }, action)
end

---Set a mousebind. If called with an already existing mousebind, it gets replaced.
---
---You must specify whether the keybind happens on button press or button release.
---
---### Example
---```lua
--- -- Set `super + left mouse button` to move a window on press
---Input.mousebind({ "super" }, "btn_left", "press", function()
---    Window.begin_move("btn_left")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param button MouseButton The mouse button used to trigger the bind
---@param edge MouseEdge "press" or "release" to trigger on button press or release
---@param action fun() The function to run when the bind is triggered
function input.mousebind(mods, button, edge, action)
    ---@diagnostic disable-next-line: redefined-local
    local edge = mouse_edge_values[edge]

    local mod_values = {}
    for _, mod in ipairs(mods) do
        table.insert(mod_values, modifier_values[mod])
    end

    client:server_streaming_request(input_service.SetMousebind, {
        modifiers = mod_values,
        button = mouse_button_values[button],
        edge = edge,
    }, action)
end

---@class KeybindDescription
---@field modifiers Modifier[]
---@field raw_code integer
---@field xkb_name string
---@field group string?
---@field description string?

---Get all keybinds along with their descriptions
---
---@return KeybindDescription[]
function input.keybind_descriptions()
    ---@type pinnacle.input.v0alpha1.KeybindDescriptionsResponse
    local descs = client:unary_request(input_service.KeybindDescriptions, {})
    local descs = descs.descriptions or {}

    local ret = {}

    for _, desc in ipairs(descs) do
        local mods = {}
        for _, mod in ipairs(desc.modifiers or {}) do
            if mod == modifier_values.shift then
                table.insert(mods, "shift")
            elseif mod == modifier_values.ctrl then
                table.insert(mods, "ctrl")
            elseif mod == modifier_values.alt then
                table.insert(mods, "alt")
            elseif mod == modifier_values.super then
                table.insert(mods, "super")
            end
        end

        desc.modifiers = mods
        table.insert(ret, desc)
    end

    return ret
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
---Input.set_xkb_config({
---    layout = "us,fr,ge",
---    options = "ctrl:swapcaps,caps:shift"
---})
---```
---
---@param xkb_config XkbConfig The new xkbconfig
function input.set_xkb_config(xkb_config)
    client:unary_request(input_service.SetXkbConfig, xkb_config)
end

---Set the keyboard's repeat rate and delay.
---
---### Example
---```lua
---Input.set_repeat_rate(100, 1000) -- Key must be held down for 1 second, then repeats 10 times per second.
---```
---
---@param rate integer The time between repeats in milliseconds
---@param delay integer The duration a key needs to be held down before repeating starts in milliseconds
function input.set_repeat_rate(rate, delay)
    client:unary_request(input_service.SetRepeatRate, {
        rate = rate,
        delay = delay,
    })
end

---@enum (key) AccelProfile
local accel_profile_values = {
    ---No pointer acceleration
    flat = input_v0alpha1.SetLibinputSettingRequest.AccelProfile.ACCEL_PROFILE_FLAT,
    ---Pointer acceleration
    adaptive = input_v0alpha1.SetLibinputSettingRequest.AccelProfile.ACCEL_PROFILE_ADAPTIVE,
}

---@enum (key) ClickMethod
local click_method_values = {
    ---Button presses are generated according to where on the device the click occurs
    button_areas = input_v0alpha1.SetLibinputSettingRequest.ClickMethod.CLICK_METHOD_BUTTON_AREAS,
    ---Button presses are generated according to the number of fingers used
    click_finger = input_v0alpha1.SetLibinputSettingRequest.ClickMethod.CLICK_METHOD_CLICK_FINGER,
}

---@enum (key) ScrollMethod
local scroll_method_values = {
    ---Never send scroll events instead of pointer motion events
    no_scroll = input_v0alpha1.SetLibinputSettingRequest.ScrollMethod.SCROLL_METHOD_NO_SCROLL,
    ---Send scroll events when two fingers are logically down on the device
    two_finger = input_v0alpha1.SetLibinputSettingRequest.ScrollMethod.SCROLL_METHOD_TWO_FINGER,
    ---Send scroll events when a finger moves along the bottom or right edge of a device
    edge = input_v0alpha1.SetLibinputSettingRequest.ScrollMethod.SCROLL_METHOD_EDGE,
    ---Send scroll events when a button is down and the device moves along a scroll-capable axis
    on_button_down = input_v0alpha1.SetLibinputSettingRequest.ScrollMethod.SCROLL_METHOD_ON_BUTTON_DOWN,
}

---@enum (key) TapButtonMap
local tap_button_map_values = {
    ---1/2/3 finger tap maps to left/right/middle
    left_right_middle = input_v0alpha1.SetLibinputSettingRequest.TapButtonMap.TAP_BUTTON_MAP_LEFT_RIGHT_MIDDLE,
    ---1/2/3 finger tap maps to left/middle/right
    left_middle_right = input_v0alpha1.SetLibinputSettingRequest.TapButtonMap.TAP_BUTTON_MAP_LEFT_MIDDLE_RIGHT,
}

---@class LibinputSettings
---@field accel_profile AccelProfile? Set pointer acceleration
---@field accel_speed number? Set pointer acceleration speed
---@field calibration_matrix integer[]?
---@field click_method ClickMethod?
---@field disable_while_typing boolean? Set whether or not to disable the pointing device while typing
---@field left_handed boolean? Set device left-handedness
---@field middle_emulation boolean?
---@field rotation_angle integer?
---@field scroll_button integer? Set the scroll button
---@field scroll_button_lock boolean? Set whether or not the scroll button is a hold or toggle
---@field scroll_method ScrollMethod?
---@field natural_scroll boolean? Set whether or not natural scroll is enabled, which reverses scroll direction
---@field tap_button_map TapButtonMap?
---@field tap_drag boolean?
---@field tap_drag_lock boolean?
---@field tap boolean?

---Set a libinput setting.
---
---This includes settings for pointer devices, like acceleration profiles, natural scroll, and more.
---
---### Example
---```lua
---Input.set_libinput_settings({
---    accel_profile = "flat",
---    natural_scroll = true,
---})
---```
---
---@param settings LibinputSettings
function input.set_libinput_settings(settings)
    for setting, value in pairs(settings) do
        if setting == "accel_profile" then
            client:unary_request(
                input_service.SetLibinputSetting,
                { [setting] = accel_profile_values[value] }
            )
        elseif setting == "calibration_matrix" then
            client:unary_request(
                input_service.SetLibinputSetting,
                { [setting] = { matrix = value } }
            )
        elseif setting == "click_method" then
            client:unary_request(
                input_service.SetLibinputSetting,
                { [setting] = click_method_values[value] }
            )
        elseif setting == "scroll_method" then
            client:unary_request(
                input_service.SetLibinputSetting,
                { [setting] = scroll_method_values[value] }
            )
        elseif setting == "tap_button_map" then
            client:unary_request(
                input_service.SetLibinputSetting,
                { [setting] = tap_button_map_values[value] }
            )
        else
            client:unary_request(input_service.SetLibinputSetting, { [setting] = value })
        end
    end
end

---Sets the current xcursor theme.
---
---Pinnacle reads `$XCURSOR_THEME` on startup to set the theme.
---This allows you to set it at runtime.
---
---@param theme string
function input.set_xcursor_theme(theme)
    client:unary_request(input_service.SetXcursor, {
        theme = theme,
    })
end

---Sets the current xcursor size.
---
---Pinnacle reads `$XCURSOR_SIZE` on startup to set the cursor size.
---This allows you to set it at runtime.
---
---@param size integer
function input.set_xcursor_size(size)
    client:unary_request(input_service.SetXcursor, {
        size = size,
    })
end

return input
