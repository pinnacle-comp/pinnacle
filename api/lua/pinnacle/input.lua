-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local defs = require("pinnacle.grpc.defs")
local input_v1 = defs.pinnacle.input.v1
local input_service = defs.pinnacle.input.v1.InputService

local modifier_values = {
    shift = input_v1.Modifier.MODIFIER_SHIFT,
    ctrl = input_v1.Modifier.MODIFIER_CTRL,
    alt = input_v1.Modifier.MODIFIER_ALT,
    super = input_v1.Modifier.MODIFIER_SUPER,
    iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,
}
require("pinnacle.util").make_bijective(modifier_values)

---@enum (key) Modifier
local mods_with_ignore_values = {
    shift = input_v1.Modifier.MODIFIER_SHIFT,
    ctrl = input_v1.Modifier.MODIFIER_CTRL,
    alt = input_v1.Modifier.MODIFIER_ALT,
    super = input_v1.Modifier.MODIFIER_SUPER,
    iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,

    ignore_shift = input_v1.Modifier.MODIFIER_SHIFT,
    ignore_ctrl = input_v1.Modifier.MODIFIER_CTRL,
    ignore_alt = input_v1.Modifier.MODIFIER_ALT,
    ignore_super = input_v1.Modifier.MODIFIER_SUPER,
    ignore_iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    ignore_iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,
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
    btn_left = 0x110,
    btn_right = 0x111,
    btn_middle = 0x112,
    btn_side = 0x113,
    btn_extra = 0x114,
    btn_forward = 0x115,
    btn_back = 0x116,
}

local button_value_to_name = {
    [0x110] = "btn_left",
    [0x111] = "btn_right",
    [0x112] = "btn_middle",
    [0x113] = "btn_side",
    [0x114] = "btn_extra",
    [0x115] = "btn_forward",
    [0x116] = "btn_back",
}

---@enum (key) Edge
local edge_values = {
    press = input_v1.Edge.EDGE_PRESS,
    release = input_v1.Edge.EDGE_RELEASE,
}
require("pinnacle.util").make_bijective(edge_values)

---Input management.
---
---This module provides utilities to set key- and mousebinds as well as change keyboard settings.
---@class Input
---@field private mouse_button_values table
local input = {
    key = require("pinnacle.input.keys"),
}
input.mouse_button_values = mouse_button_values

---@class Bind
---@field mods Modifier[]
---@field bind_layer string?
---@field group string? The group to place this keybind in. Used for the keybind list.
---@field description string? The description of this keybind. Used for the keybind list.
---@field quit boolean?
---@field reload_config boolean?

---@class Keybind : Bind
---@field key string|Key
---@field on_press fun()?
---@field on_release fun()?

---@param kb Keybind
local function keybind_inner(kb)
    local key_code = nil
    local xkb_name = nil

    if type(kb.key) == "number" then
        key_code = kb.key
    elseif type(kb.key) == "string" then
        xkb_name = kb.key
    end

    local modifs = {}
    local ignore_modifs = {}
    for _, mod in ipairs(kb.mods) do
        if string.match(mod, "ignore") then
            table.insert(ignore_modifs, modifier_values[mod])
        else
            table.insert(modifs, modifier_values[mod])
        end
    end

    local response, err = client:unary_request(input_service.Bind, {
        bind = {
            mods = modifs,
            ignore_mods = ignore_modifs,
            layer_name = kb.bind_layer,
            group = kb.group,
            description = kb.description,
            key = {
                key_code = key_code,
                xkb_name = xkb_name,
            },
        },
    })

    if err then
        log:error(err)
        return
    end

    ---@cast response pinnacle.input.v1.BindResponse

    local bind_id = response.bind_id or 0

    if kb.quit then
        local _, err = client:unary_request(input_service.SetQuitBind, {
            bind_id = bind_id,
        })
        return
    end

    if kb.reload_config then
        local _, err = client:unary_request(input_service.SetReloadConfigBind, {
            bind_id = bind_id,
        })
        return
    end

    local err = client:server_streaming_request(input_service.KeybindStream, {
        bind_id = bind_id,
    }, function(response)
        ---@cast response pinnacle.input.v1.KeybindStreamResponse
        if response.edge == edge_values.press then
            if kb.on_press then
                kb.on_press()
            end
        elseif response.edge == edge_values.release then
            if kb.on_release then
                kb.on_release()
            end
        end
    end)

    if err then
        log:error(err)
        return
    end
end

---Sets a keybind.
---
---This function can be called in two ways:
---1. As `Input.keybind(mods, key, on_press, bind_info?)`
---2. As `Input.keybind(<Keybind table>)`
---
---Calling this with a `Keybind` table gives you more options, including the ability to assign a bind layer
---to the keybind or set it to happen on release instead of press.
---
---When calling using the first way, you must provide three arguments:
---
--- - `mods`: An array of `Modifier`s. If you don't want any, provide an empty table.
--- - `key`: The key that will trigger `action`. You can provide three types of key:
---     - Something from the `Key` table in `Input.key`, which lists every xkbcommon key. The naming pattern is the xkbcommon key without the `KEY_` prefix, unless that would make it start with a number or the reserved lua keyword `function`, in which case the `KEY_` prefix is included.
---     - A single character representing your key. This can be something like "g", "$", "~", "1", and so on.
---     - A string of the key's name. This is the name of the xkbcommon key without the `KEY_` prefix.
--- - `on_press`: The function that will be run when the keybind is pressed.
---
---It is important to note that `"a"` is different than `"A"`. Similarly, `key.a` is different than `key.A`.
---Usually, it's best to use the non-modified key to prevent confusion and unintended behavior.
---
---Similar principles apply when calling with a `Keybind` table.
---
---#### Ignoring Modifiers
---Normally, modifiers that are not specified will require the bind to not have them held down.
---You can ignore this by adding the corresponding `"ignore_*"` modifier.
---
---#### Descriptions
---You can specify a group and description for the bind.
---This will be used to categorize the bind in the bind overlay and provide a description.
---
---#### Example
---```lua
--- -- Set `super + Return` to open Alacritty
---Input.keybind({ "super" }, Input.key.Return, function()
---    Process.spawn("alacritty")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param key Key | string The key used to trigger the bind
---@param on_press fun() The function to run when the bind is triggered
---@param bind_info { group: string?, description: string? }?
---
---@overload fun(keybind: Keybind)
function input.keybind(mods, key, on_press, bind_info)
    local kb

    if mods.key then
        kb = mods
    else
        kb = {
            mods = mods,
            key = key,
            on_press = on_press,
            group = bind_info and bind_info.group,
            description = bind_info and bind_info.description,
        }
    end

    keybind_inner(kb)
end

---@class Mousebind : Bind
---@field button MouseButton
---@field on_press fun()?
---@field on_release fun()?

---@param mb Mousebind
local function mousebind_inner(mb)
    local modifs = {}
    local ignore_modifs = {}
    for _, mod in ipairs(mb.mods) do
        if string.match(mod, "ignore") then
            table.insert(ignore_modifs, modifier_values[mod])
        else
            table.insert(modifs, modifier_values[mod])
        end
    end

    local response, err = client:unary_request(input_service.Bind, {
        bind = {
            mods = modifs,
            ignore_mods = ignore_modifs,
            layer_name = mb.bind_layer,
            group = mb.group,
            description = mb.description,
            mouse = {
                button = mouse_button_values[mb.button],
            },
        },
    })

    if err then
        log:error(err)
        return
    end

    ---@cast response pinnacle.input.v1.BindResponse

    local bind_id = response.bind_id or 0

    if mb.quit then
        local _, err = client:unary_request(input_service.SetQuitBind, {
            bind_id = bind_id,
        })
        return
    end

    if mb.reload_config then
        local _, err = client:unary_request(input_service.SetReloadConfigBind, {
            bind_id = bind_id,
        })
        return
    end

    local err = client:server_streaming_request(input_service.MousebindStream, {
        bind_id = bind_id,
    }, function(response)
        ---@cast response pinnacle.input.v1.MousebindStreamResponse
        if response.edge == edge_values.press then
            if mb.on_press then
                mb.on_press()
            end
        elseif response.edge == edge_values.release then
            if mb.on_release then
                mb.on_release()
            end
        end
    end)

    if err then
        log:error(err)
        return
    end
end

---Sets a mousebind.
---
---This function can be called in two ways:
---1. As `Input.mousebind(mods, button, on_press, bind_info?)`
---2. As `Input.mousebind(<Mousebind table>)`
---
---Calling this with a `Mousebind` table gives you more options, including the ability to assign a bind layer
---to the keybind or set it to happen on release instead of press.
---
---When calling using the first way, you must provide three arguments:
---
--- - `mods`: An array of `Modifier`s. If you don't want any, provide an empty table.
--- - `button`: The mouse button.
--- - `on_press`: The function that will be run when the button is pressed.
---
---#### Ignoring Modifiers
---Normally, modifiers that are not specified will require the bind to not have them held down.
---You can ignore this by adding the corresponding `"ignore_*"` modifier.
---
---#### Descriptions
---You can specify a group and description for the bind.
---This will be used to categorize the bind in the bind overlay and provide a description.
---
---#### Example
---```lua
--- -- Set `super + left mouse button` to move a window on press
---Input.mousebind({ "super" }, "btn_left", "press", function()
---    Window.begin_move("btn_left")
---end)
---```
---
---@param mods Modifier[] The modifiers that need to be held down for the bind to trigger
---@param button MouseButton The mouse button used to trigger the bind
---@param on_press fun() The function to run when the bind is triggered
---@param bind_info { group: string?, description: string? }?
---
---@overload fun(mousebind: Mousebind)
function input.mousebind(mods, button, on_press, bind_info)
    local mb

    if mods.button then
        mb = mods
    else
        mb = {
            mods = mods,
            button = button,
            on_press = on_press,
            group = bind_info and bind_info.group,
            description = bind_info and bind_info.description,
        }
    end

    mousebind_inner(mb)
end

---Enters the bind layer `layer`, or the default layer if `layer` is nil.
---
---@param layer string?
function input.enter_bind_layer(layer)
    local _, err = client:unary_request(input_service.EnterBindLayer, {
        layer_name = layer,
    })
end

---@class BindInfo
---@field mods Modifier[]
---@field ignore_mods Modifier[]
---@field bind_layer string?
---@field group string?
---@field description string?
---@field kind BindInfoKind

---@class BindInfoKind
---@field key { key_code: integer, xkb_name: string }?
---@field mouse { button: MouseButton }?

---Gets all binds and their information.
---
---@return BindInfo[]
function input.bind_infos()
    local response, err = client:unary_request(input_service.GetBindInfos, {})

    if err then
        log:error(err)
        return {}
    end

    ---@cast response pinnacle.input.v1.GetBindInfosResponse

    ---@type BindInfo[]
    local ret = {}

    local infos = response.bind_infos or {}

    for _, desc in ipairs(infos) do
        local info = desc.bind
        if not info then
            goto continue
        end

        ---@type Modifier[]
        local mods = {}
        for _, mod in ipairs(info.mods or {}) do
            table.insert(mods, modifier_values[mod])
        end

        ---@type Modifier[]
        local ignore_mods = {}
        for _, mod in ipairs(info.ignore_mods or {}) do
            table.insert(ignore_mods, modifier_values[mod])
        end

        ---@type BindInfoKind
        local bind_kind = {}
        if info.key then
            bind_kind.key = {
                key_code = info.key.key_code,
                xkb_name = info.key.xkb_name,
            }
        elseif info.mouse then
            bind_kind.mouse = {
                button = button_value_to_name[info.mouse.button],
            }
        end

        local bind_layer = info.layer_name
        local group = info.group
        local description = info.description

        ---@type BindInfo
        local bind_info = {
            mods = mods,
            ignore_mods = ignore_mods,
            bind_layer = bind_layer,
            group = group,
            description = description,
            kind = bind_kind,
        }

        table.insert(ret, bind_info)

        ::continue::
    end

    return ret
end

---@class XkbConfig
---@field rules string?
---@field model string?
---@field layout string?
---@field variant string?
---@field options string?

---Sets the xkbconfig for your keyboard.
---
---Read `xkeyboard-config(7)` for more information.
---
---#### Example
---```lua
---Input.set_xkb_config({
---    layout = "us,fr,ge",
---    options = "ctrl:swapcaps,caps:shift"
---})
---```
---
---@param xkb_config XkbConfig The new xkbconfig
function input.set_xkb_config(xkb_config)
    local _, err = client:unary_request(input_service.SetXkbConfig, xkb_config)

    if err then
        log:error(err)
    end
end

---Sets the keyboard's repeat rate and delay.
---
---#### Example
---```lua
---Input.set_repeat_rate(100, 1000) -- Key must be held down for 1 second, then repeats 10 times per second.
---```
---
---@param rate integer The time between repeats in milliseconds
---@param delay integer The duration a key needs to be held down before repeating starts in milliseconds
function input.set_repeat_rate(rate, delay)
    local _, err = client:unary_request(input_service.SetRepeatRate, {
        rate = rate,
        delay = delay,
    })

    if err then
        log:error(err)
    end
end

---Sets the current xcursor theme.
---
---Pinnacle reads `$XCURSOR_THEME` on startup to set the theme.
---This allows you to set it at runtime.
---
---@param theme string
function input.set_xcursor_theme(theme)
    local _, err = client:unary_request(input_service.SetXcursor, {
        theme = theme,
    })

    if err then
        log:error(err)
    end
end

---Sets the current xcursor size.
---
---Pinnacle reads `$XCURSOR_SIZE` on startup to set the cursor size.
---This allows you to set it at runtime.
---
---@param size integer
function input.set_xcursor_size(size)
    local _, err = client:unary_request(input_service.SetXcursor, {
        size = size,
    })

    if err then
        log:error(err)
    end
end

---@class InputSignal Signals related to input events.
---@field device_added fun(device: pinnacle.input.libinput.DeviceHandle)? A new input device was connected.

local signal_name_to_SignalName = {
    device_added = "InputDeviceAdded",
}

---Connects to an input signal.
---
---`signals` is a table containing the signal(s) you want to connect to along with
---a corresponding callback that will be called when the signal is signalled.
---
---This function returns a table of signal handles with each handle stored at the same key used
---to connect to the signal. See `SignalHandles` for more information.
---
---# Example
---```lua
---Input.connect_signal({
---    device_added = function(device)
---        print("Device connected", device:name())
---    end
---})
---```
---@param signals InputSignal The signal you want to connect to
---
---@return SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see SignalHandles.disconnect_all - To disconnect from these signals
function input.connect_signal(signals)
    ---@diagnostic disable-next-line: invisible
    local handles = require("pinnacle.signal").handles.new({})

    for signal, callback in pairs(signals) do
        require("pinnacle.signal").add_callback(signal_name_to_SignalName[signal], callback)
        local handle =
            ---@diagnostic disable-next-line: invisible
            require("pinnacle.signal").handle.new(signal_name_to_SignalName[signal], callback)
        handles[signal] = handle
    end

    return handles
end

return input
