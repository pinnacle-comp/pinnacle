-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local defs = require("pinnacle.grpc.defs")
local input_v1 = defs.pinnacle.input.v1

local modifier_values = {
    shift = input_v1.Modifier.MODIFIER_SHIFT,
    ctrl = input_v1.Modifier.MODIFIER_CTRL,
    alt = input_v1.Modifier.MODIFIER_ALT,
    super = input_v1.Modifier.MODIFIER_SUPER,
    iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,
}
require("pinnacle.util").make_bijective(modifier_values)

---A keyboard modifier for use in binds.
---
---Binds can be configured to require certain keyboard modifiers to be held down to trigger.
---For example, a bind with `{ "super", "ctrl" }` requires both the super and control keys
---to be held down.
---
---Normally, modifiers must be in the exact same state as passed in to trigger a bind.
---This means if you use `"super"` in a bind, *only* super must be held down; holding
---down any other modifier will invalidate the bind.
---
---To circumvent this, you can ignore certain modifiers by adding the respective `"ignore_*"` modifier.
---@enum (key) pinnacle.input.Mod
local mods_with_ignore_values = {
    ---The shift key.
    shift = input_v1.Modifier.MODIFIER_SHIFT,
    ---The control key.
    ctrl = input_v1.Modifier.MODIFIER_CTRL,
    ---The alt key.
    alt = input_v1.Modifier.MODIFIER_ALT,
    ---The super key.
    super = input_v1.Modifier.MODIFIER_SUPER,
    ---The IsoLevel3Shift modifier.
    iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    ---The IsoLevel5Shift modifier.
    iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,

    ---Ignore the shift key.
    ignore_shift = input_v1.Modifier.MODIFIER_SHIFT,
    ---Ignore the control key.
    ignore_ctrl = input_v1.Modifier.MODIFIER_CTRL,
    ---Ignore the alt key.
    ignore_alt = input_v1.Modifier.MODIFIER_ALT,
    ---Ignore the super key.
    ignore_super = input_v1.Modifier.MODIFIER_SUPER,
    ---Ignore the IsoLevel3Shift modifier.
    ignore_iso_level3_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL3_SHIFT,
    ---Ignore the IsoLevel5Shift modifier.
    ignore_iso_level5_shift = input_v1.Modifier.MODIFIER_ISO_LEVEL5_SHIFT,
}

---A mouse button.
---@enum (key) pinnacle.input.MouseButton
local mouse_button_values = {
    ---The left mouse button.
    [1] = 0x110,
    ---The right mouse button.
    [2] = 0x111,
    ---The middle mouse button.
    [3] = 0x112,
    ---The side mouse button.
    [4] = 0x113,
    ---The extra mouse button.
    [5] = 0x114,
    ---The forward mouse button.
    [6] = 0x115,
    ---The back mouse button.
    [7] = 0x116,
    ---The left mouse button.
    btn_left = 0x110,
    ---The right mouse button.
    btn_right = 0x111,
    ---The middle mouse button.
    btn_middle = 0x112,
    ---The side mouse button.
    btn_side = 0x113,
    ---The extra mouse button.
    btn_extra = 0x114,
    ---The forward mouse button.
    btn_forward = 0x115,
    ---The back mouse button.
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

local edge_values = {
    press = input_v1.Edge.EDGE_PRESS,
    release = input_v1.Edge.EDGE_RELEASE,
}
require("pinnacle.util").make_bijective(edge_values)

---Input management.
---
---This module provides utilities to set key- and mousebinds as well as change keyboard settings.
---@class pinnacle.input
---@field private mouse_button_values table
local input = {
    ---Keycodes for every key.
    key = require("pinnacle.input.keys"),
}
input.mouse_button_values = mouse_button_values

---An input bind.
---@class pinnacle.input.Bind
---The modifiers that need to be pressed for this bind to trigger.
---@field mods pinnacle.input.Mod[]
---The layer that this bind is assigned.
---@field bind_layer string?
---The group to place this keybind in. Used for the keybind list.
---@field group string?
---The description of this keybind. Used for the keybind list.
---@field description string?
---Sets this bind as a quit bind.
---@field quit boolean?
---Sets this bind as a reload config bind.
---@field reload_config boolean?
---Allows this bind to trigger when the session is locked.
---@field allow_when_locked boolean?

---A keybind.
---@class pinnacle.input.Keybind : pinnacle.input.Bind
---The key that will trigger this bind.
---@field key string|pinnacle.input.Key
---An action that is run when the keybind is pressed.
---@field on_press fun()?
---An action that is run when the keybind is released.
---@field on_release fun()?

---@param kb pinnacle.input.Keybind
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
            table.insert(ignore_modifs, mods_with_ignore_values[mod])
        else
            table.insert(modifs, mods_with_ignore_values[mod])
        end
    end

    local response, err = client:pinnacle_input_v1_InputService_Bind({
        bind = {
            mods = modifs,
            ignore_mods = ignore_modifs,
            layer_name = kb.bind_layer,
            properties = {
                group = kb.group,
                description = kb.description,
                quit = kb.quit,
                reload_config = kb.reload_config,
                allow_when_locked = kb.allow_when_locked,
            },
            key = {
                key_code = key_code,
                xkb_name = xkb_name,
            },
        },
    })

    if err then
        log.error(err)
        return
    end

    assert(response)

    local bind_id = response.bind_id or 0

    local err = client:pinnacle_input_v1_InputService_KeybindStream({
        bind_id = bind_id,
    }, function(response)
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

    if kb.on_press then
        local _, err = client:pinnacle_input_v1_InputService_KeybindOnPress({
            bind_id = bind_id,
        })
    end

    if err then
        log.error(err)
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
---@param mods pinnacle.input.Mod[] The modifiers that need to be held down for the bind to trigger
---@param key pinnacle.input.Key | string The key used to trigger the bind
---@param on_press fun() The function to run when the bind is triggered
---@param bind_info { group: string?, description: string? }? An optional group and description that is displayed in the bind overlay.
---
---@overload fun(keybind: pinnacle.input.Keybind)
function input.keybind(mods, key, on_press, bind_info)
    ---@type pinnacle.input.Keybind
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

---A mousebind.
---@class pinnacle.input.Mousebind : pinnacle.input.Bind
---The mouse button that will trigger this bind.
---@field button pinnacle.input.MouseButton
---An action that will be run when the mousebind is pressed.
---@field on_press fun()?
---An action that will be run when the mousebind is released.
---@field on_release fun()?

---@param mb pinnacle.input.Mousebind
local function mousebind_inner(mb)
    local modifs = {}
    local ignore_modifs = {}
    for _, mod in ipairs(mb.mods) do
        if string.match(mod, "ignore") then
            table.insert(ignore_modifs, mods_with_ignore_values[mod])
        else
            table.insert(modifs, mods_with_ignore_values[mod])
        end
    end

    local response, err = client:pinnacle_input_v1_InputService_Bind({
        bind = {
            mods = modifs,
            ignore_mods = ignore_modifs,
            layer_name = mb.bind_layer,
            properties = {
                group = mb.group,
                description = mb.description,
                quit = mb.quit,
                reload_config = mb.reload_config,
                allow_when_locked = mb.allow_when_locked,
            },
            mouse = {
                button = mouse_button_values[mb.button],
            },
        },
    })

    if err then
        log.error(err)
        return
    end

    assert(response)

    local bind_id = response.bind_id or 0

    local err = client:pinnacle_input_v1_InputService_MousebindStream({
        bind_id = bind_id,
    }, function(response)
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

    if mb.on_press then
        local _, err = client:pinnacle_input_v1_InputService_MousebindOnPress({
            bind_id = bind_id,
        })
    end

    if err then
        log.error(err)
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
---@param mods pinnacle.input.Mod[] The modifiers that need to be held down for the bind to trigger
---@param button pinnacle.input.MouseButton The mouse button used to trigger the bind
---@param on_press fun() The function to run when the bind is triggered
---@param bind_info { group: string?, description: string? }? An optional group and description that will be displayed in the bind overlay.
---
---@overload fun(mousebind: pinnacle.input.Mousebind)
function input.mousebind(mods, button, on_press, bind_info)
    ---@type pinnacle.input.Mousebind
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
---@param layer string? The bind layer.
function input.enter_bind_layer(layer)
    local _, err = client:pinnacle_input_v1_InputService_EnterBindLayer({
        layer_name = layer,
    })
end

---Bind information.
---
---Mainly used for the bind overlay.
---@class pinnacle.input.BindInfo
---The bind's modifiers.
---@field mods pinnacle.input.Mod[]
---The bind's ignored modifiers.
---@field ignore_mods pinnacle.input.Mod[]
---The bind's layer.
---@field bind_layer string?
---The bind's group. Empty if it is not in one.
---@field group string
---The bind's description. Empty if it does not have one.
---@field description string
---Whether this bind is a quit bind.
---@field quit boolean
---Whether this bind is a reload config bind.
---@field reload_config boolean
---Whether this bind is allowed when the session is locked.
---@field allow_when_locked boolean
---What kind of bind this is.
---@field kind pinnacle.input.BindInfoKind

---The kind of a bind.
---@class pinnacle.input.BindInfoKind
---This is a keybind.
---@field key { key_code: integer, xkb_name: string }?
---This is a mousebind.
---@field mouse { button: pinnacle.input.MouseButton }?

---Gets all binds and their information.
---
---@return pinnacle.input.BindInfo[]
function input.bind_infos()
    local response, err = client:pinnacle_input_v1_InputService_GetBindInfos({})

    if err then
        log.error(err)
        return {}
    end

    assert(response)

    ---@type pinnacle.input.BindInfo[]
    local ret = {}

    local infos = response.bind_infos or {}

    for _, desc in ipairs(infos) do
        local info = desc.bind
        if not info then
            goto continue
        end

        ---@type pinnacle.input.Mod[]
        local mods = {}
        for _, mod in ipairs(info.mods or {}) do
            table.insert(mods, modifier_values[mod])
        end

        ---@type pinnacle.input.Mod[]
        local ignore_mods = {}
        for _, mod in ipairs(info.ignore_mods or {}) do
            table.insert(ignore_mods, modifier_values[mod])
        end

        ---@type pinnacle.input.BindInfoKind
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
        local group = info.properties.group or ""
        local description = info.properties.description or ""
        local quit = info.properties.quit or false
        local reload_config = info.properties.reload_config or false
        local allow_when_locked = info.properties.allow_when_locked or false

        ---@type pinnacle.input.BindInfo
        local bind_info = {
            mods = mods,
            ignore_mods = ignore_mods,
            bind_layer = bind_layer,
            group = group,
            description = description,
            quit = quit,
            reload_config = reload_config,
            allow_when_locked = allow_when_locked,
            kind = bind_kind,
        }

        table.insert(ret, bind_info)

        ::continue::
    end

    return ret
end

---Xkeyboard config options.
---
---See `xkeyboard-config(7)` for more information.
---@class pinnacle.input.XkbConfig
---Files of rules to be used for keyboard mapping composition.
---@field rules string?
---Name of the model of your keyboard type.
---@field model string?
---Layout(s) you intend to use.
---@field layout string?
---Variant(s) of the layout you intend to use.
---@field variant string?
---Extra xkb configuration options.
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
---@param xkb_config pinnacle.input.XkbConfig The new xkbconfig
function input.set_xkb_config(xkb_config)
    local _, err = client:pinnacle_input_v1_InputService_SetXkbConfig({
        rules = xkb_config.rules,
        model = xkb_config.model,
        layout = xkb_config.layout,
        variant = xkb_config.variant,
        options = xkb_config.options,
    })

    if err then
        log.error(err)
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
    local _, err = client:pinnacle_input_v1_InputService_SetRepeatRate({
        rate = rate,
        delay = delay,
    })

    if err then
        log.error(err)
    end
end

---Sets the XKB keymap.
---
---#### Examples
---```lua
---Input.set_xkb_keymap("keymap here...")
---
----- From a file
---Input.set_xkb_keymap(io.open("/path/to/keymap.xkb"):read("*a"))
---```
---
---@param keymap string The keymap to set.
function input.set_xkb_keymap(keymap)
    local _, err = client:pinnacle_input_v1_InputService_SetXkbKeymap({
        keymap = keymap,
    })

    if err then
        log.error(err)
    end
end

---Cycles the current XKB layout forward.
function input.cycle_xkb_layout_forward()
    local _, err = client:pinnacle_input_v1_InputService_SwitchXkbLayout({
        next = {},
    })

    if err then
        log.error(err)
    end
end

---Cycles the current XKB layout backward.
function input.cycle_xkb_layout_backward()
    local _, err = client:pinnacle_input_v1_InputService_SwitchXkbLayout({
        prev = {},
    })

    if err then
        log.error(err)
    end
end

---Switches the current XKB layout to the one at the provided `index`.
---
---Fails if the index is out of bounds.
---
---@param index integer The index of the layout to switch to.
function input.switch_xkb_layout(index)
    local _, err = client:pinnacle_input_v1_InputService_SwitchXkbLayout({
        index = index,
    })

    if err then
        log.error(err)
    end
end

---Sets the current xcursor theme.
---
---Pinnacle reads `$XCURSOR_THEME` on startup to set the theme.
---This allows you to set it at runtime.
---
---@param theme string The name of the xcursor theme.
function input.set_xcursor_theme(theme)
    local _, err = client:pinnacle_input_v1_InputService_SetXcursor({
        theme = theme,
    })

    if err then
        log.error(err)
    end
end

---Sets the current xcursor size.
---
---Pinnacle reads `$XCURSOR_SIZE` on startup to set the cursor size.
---This allows you to set it at runtime.
---
---@param size integer The new size of the cursor.
function input.set_xcursor_size(size)
    local _, err = client:pinnacle_input_v1_InputService_SetXcursor({
        size = size,
    })

    if err then
        log.error(err)
    end
end

---@class pinnacle.input.InputSignal Signals related to input events.
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
---@param signals pinnacle.input.InputSignal The signal you want to connect to
---
---@return pinnacle.signal.SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see pinnacle.signal.SignalHandles.disconnect_all - To disconnect from these signals
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
