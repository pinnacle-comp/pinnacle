-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@class InputModule
local input_module = {
    keys = require("keys"),
}

---Set a keybind. If called with an already existing keybind, it gets replaced.
---
---### Example
---
---```lua
----- Set `Super + Return` to open Alacritty
---input.keybind({ "Super" }, input.keys.Return, function()
---    process.spawn("Alacritty")
---end)
---```
---@param key Keys The key for the keybind.
---@param modifiers (Modifier)[] Which modifiers need to be pressed for the keybind to trigger.
---@param action fun() What to do.
function input_module.keybind(modifiers, key, action)
    table.insert(CallbackTable, action)
    SendMsg({
        SetKeybind = {
            modifiers = modifiers,
            key = key,
            callback_id = #CallbackTable,
        },
    })
end

return input_module
