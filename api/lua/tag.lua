-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

local tag = {}

---Add tags.
---
---If you need to add the names as a table, use `tag.add_table` instead.
---
---# Example
---
---```lua
---local output = output.get_by_name("DP-1")
---if output ~= nil then
---    tag.add(output, "1", "2", "3", "4", "5") -- Add tags with names 1-5
---end
---```
---@param output Output The output you want these tags to be added to.
---@param ... string The names of the new tags you want to add.
function tag.add(output, ...)
    local tags = table.pack(...)
    tags["n"] = nil

    SendMsg({
        AddTags = {
            output_name = output.name,
            tags = tags,
        },
    })
end

---Like `tag.add`, but with a table of strings instead.
---
---# Example
---
---```lua
---local tags = { "Terminal", "Browser", "Mail", "Gaming", "Potato" }
---local output = output.get_by_name("DP-1")
---if output ~= nil then
---    tag.add(output, tags) -- Add tags with the names above
---end
---```
---@param output Output The output you want these tags to be added to.
---@param tags string[] The names of the new tags you want to add, as a table.
function tag.add_table(output, tags)
    SendMsg({
        AddTags = {
            output_name = output.name,
            tags = tags,
        },
    })
end

---Toggle a tag on the specified output. If `output` isn't specified, toggle it on the currently focused output instead.
---
---# Example
---
---```lua
----- Assuming all tags are toggled off...
---local op = output.get_by_name("DP-1")
---tag.toggle("1", op)
---tag.toggle("2", op)
----- will cause windows on both tags 1 and 2 to be displayed at the same time.
---```
---@param name string The name of the tag.
---@param output Output? The output.
function tag.toggle(name, output)
    if output ~= nil then
        SendMsg({
            ToggleTag = {
                output_name = output.name,
                tag_name = name,
            },
        })
    else
        local op = require("output").get_focused()
        if op ~= nil then
            SendMsg({
                ToggleTag = {
                    output_name = op.name,
                    tag_name = name,
                },
            })
        end
    end
end

---Switch to a tag on the specified output, deactivating any other active tags on it.
---If `output` is not specified, this uses the currently focused output instead.
---
---This is used to replicate what a traditional workspace is on some other Wayland compositors.
---
---# Example
---
---```lua
---tag.switch_to("3") -- Switches to and displays *only* windows on tag 3
---```
---@param name string The name of the tag.
---@param output Output? The output.
function tag.switch_to(name, output)
    if output ~= nil then
        SendMsg({
            SwitchToTag = {
                output_name = output.name,
                tag_name = name,
            },
        })
    else
        local op = require("output").get_focused()
        if op ~= nil then
            SendMsg({
                SwitchToTag = {
                    output_name = op.name,
                    tag_name = name,
                },
            })
        end
    end
end

return tag
