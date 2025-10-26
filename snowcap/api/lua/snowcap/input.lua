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

---A Key event.
---@class snowcap.input.KeyEvent
---@field key snowcap.Key Key Symbol.
---@field mods snowcap.input.Modifiers Currently active modifiers.
---@field pressed boolean True if the key is currently pressed, false on release.
---@field captured boolean True if the event was flagged as Captured by a widget.
---@field text? string Text produced by the event, if any.

return input
