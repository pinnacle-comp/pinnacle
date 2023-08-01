-- SPDX-License-Identifier: GPL-3.0-or-later

---Input and keybinds
---@module InputModule
local input_module = {}

---Set a keybind. If called with an already existing keybind, it gets replaced.
---
---### Example
---    -- Set `Super + Return` to open Alacritty
---    input.keybind({ "Super" }, input.keys.Return, function()
---        process.spawn("Alacritty")
---    end)
---@tparam Modifier[] modifiers Which modifiers need to be pressed for the keybind to trigger.
---@tparam Keys key The key for the keybind.
---@tparam function action What to do.
function input_module.keybind(modifiers, key, action) end
