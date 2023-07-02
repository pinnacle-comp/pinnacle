-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

local tag = {}

---Add tags.
---
---If you need to add the strings in a table, use `tag.add_table` instead.
---
---# Example
---
---```lua
---tag.add("1", "2", "3", "4", "5") -- Add tags with names 1-5
---```
---@param ... string The names of the new tags you want to add.
function tag.add(...)
    local tags = table.pack(...)
    tags["n"] = nil

    SendMsg({
        AddTags = {
            tags = tags,
        },
    })
end

---Like `tag.add`, but with a table of strings instead.
---@param tags string[] The names of the new tags you want to add, as a table.
function tag.add_table(tags)
    SendMsg({
        AddTags = {
            tags = tags,
        },
    })
end

---Toggle a tag's display.
---@param name string The name of the tag.
function tag.toggle(name)
    SendMsg({
        ToggleTag = {
            tag_id = name,
        },
    })
end

---Switch to a tag, deactivating any other active tags.
---@param name string The name of the tag.
function tag.switch_to(name)
    SendMsg({
        SwitchToTag = {
            tag_id = name,
        },
    })
end

return tag
