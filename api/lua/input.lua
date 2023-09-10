-- SPDX-License-Identifier: GPL-3.0-or-later

---@class InputModule
local input_module = {
    keys = require("keys"),
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

return input_module
