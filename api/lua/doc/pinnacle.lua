-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---The configuration entry point.
---@module Pinnacle
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
