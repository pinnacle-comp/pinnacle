-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local input = {
    key = require("snowcap.input.keys"),
}

---@class snowcap.input.Modifiers
---@field shift boolean
---@field ctrl boolean
---@field alt boolean
---@field super boolean

return input
