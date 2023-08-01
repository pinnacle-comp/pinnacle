-- SPDX-License-Identifier: GPL-3.0-or-later

---The configuration entry point.
---@module PinnacleModule
local pinnacle = {
    ---Input and keybinds
    ---@tparam InputModule input
    ---@see InputModule
    input = nil,
    ---Window management
    ---@tparam WindowModule window
    ---@see WindowModule
    window = nil,
    ---Process management
    ---@tparam ProcessModule process
    ---@see ProcessModule
    process = nil,
    ---Tag management
    ---@tparam TagModule tag
    ---@see TagModule
    tag = nil,
    ---Output management
    ---@tparam OutputModule output
    ---@see OutputModule
    output = nil,
}

---Quit Pinnacle.
function pinnacle.quit() end

---Configure Pinnacle.
---
---You should put mostly eveything into the config_func to avoid invalid state.
---@tparam function config_func A function that takes in the `Pinnacle` table.
function pinnacle.setup(config_func) end
