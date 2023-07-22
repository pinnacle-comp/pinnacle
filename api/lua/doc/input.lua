-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

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
---@tparam Keys key The key for the keybind.
---@tparam Modifier[] modifiers Which modifiers need to be pressed for the keybind to trigger.
---@tparam function action What to do.
function input_module.keybind(modifiers, key, action) end
