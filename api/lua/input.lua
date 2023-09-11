-- SPDX-License-Identifier: GPL-3.0-or-later

---@nodoc TODO: add enum, alias, and type capabilities to ldoc_gen
---@enum MouseButton
local buttons = {
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

---Input management.
---
---This module provides utilities to set keybinds.
---@class InputModule
local input_module = {
    --- A table with every key provided by xkbcommon.
    keys = require("keys"),
    --- A table with mouse button codes. You can use indexes (1, 2, and 3 are left, right, and middle)
    --- or keyed values (buttons.left, buttons.right, etc.).
    buttons = buttons,
}

---Set a keybind. If called with an already existing keybind, it gets replaced.
---
---You must provide three arguments:
---
--- - `modifiers`: An array of `Modifier`s. If you don't want any, provide an empty table.
--- - `key`: The key that will trigger `action`. You can provide three types of key:
---     - Something from the `Keys` table in `input.keys`, which lists every xkbcommon key. The naming pattern is the xkbcommon key without the `KEY_` prefix, unless that would make it start with a number or the reserved lua keyword `function`, in which case the `KEY_` prefix is included.
---     - A single character representing your key. This can be something like "g", "$", "~", "1", and so on.
---     - A string of the key's name. This is the name of the xkbcommon key without the `KEY_` prefix.
--- - `action`: The function that will be run when the keybind is pressed.
---
---It is important to note that `"a"` is different than `"A"`. Similarly, `keys.a` is different than `keys.A`.
---Usually, it's best to use the non-modified key to prevent confusion and unintended behavior.
---
---```lua
---input.keybind({ "Shift" }, "a", function() end) -- This is preferred
---input.keybind({ "Shift" }, "A", function() end) -- over this
---
--- -- And in fact, this keybind won't work at all because it expects no modifiers,
--- -- but you can't get "A" without using `Shift`.
---input.keybind({}, "A", function() end)
---```
---
---### Example
---
---```lua
--- -- Set `Super + Return` to open Alacritty
---input.keybind({ "Super" }, input.keys.Return, function()
---    process.spawn("Alacritty")
---end)
---```
---@param key Keys|string The key for the keybind.
---@param modifiers (Modifier)[] Which modifiers need to be pressed for the keybind to trigger.
---@param action fun() What to do.
function input_module.keybind(modifiers, key, action)
    table.insert(CallbackTable, action)

    local k = {}

    if type(key) == "string" then
        k.String = key
    else
        k.Int = key
    end

    SendMsg({
        SetKeybind = {
            modifiers = modifiers,
            key = k,
            callback_id = #CallbackTable,
        },
    })
end

---Set a mousebind. If called with an already existing mousebind, it gets replaced.
---
---The mousebind can happen either on button press or release, so you must specify
---which edge you desire.
---
---@param modifiers (Modifier)[] The modifiers that need to be held for the mousebind to trigger.
---@param button MouseButton The button that needs to be pressed or released.
---@param edge "Press"|"Release" Whether or not to trigger `action` on button press or release.
---@param action fun() The function to run.
function input_module.mousebind(modifiers, button, edge, action)
    table.insert(CallbackTable, action)

    SendMsg({
        SetMousebind = {
            modifiers = modifiers,
            button = button,
            edge = edge,
            callback_id = #CallbackTable,
        },
    })
end

return input_module
