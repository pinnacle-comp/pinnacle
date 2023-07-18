-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

local tag = {}

---@alias Layout
---| "MasterStack" # One master window on the left with all other windows stacked to the right.
---| "Dwindle" # Windows split in half towards the bottom right corner.
---| "Spiral" # Windows split in half in a spiral.
---| "CornerTopLeft" # One main corner window in the top left with a column of windows on the right and a row on the bottom.
---| "CornerTopRight" # One main corner window in the top right with a column of windows on the left and a row on the bottom.
---| "CornerBottomLeft" # One main corner window in the bottom left with a column of windows on the right and a row on the top.
---| "CornerBottomRight" # One main corner window in the bottom right with a column of windows on the left and a row on the top.

---@class Tag
---@field private id integer The internal id of this tag.
local tg = {}

---@param props Tag
---@return Tag
local function new_tag(props)
    -- Copy functions over
    for k, v in pairs(tg) do
        props[k] = v
    end

    return props
end

---Get this tag's active status.
---@return boolean active True if the tag is active, otherwise false.
function tg:active()
    SendRequest({
        GetTagActive = {
            tag_id = self.id,
        },
    })

    local response = ReadMsg()
    local active = response.RequestResponse.response.TagActive.active
    return active
end

function tg:name()
    SendRequest({
        GetTagName = {
            tag_id = self.id,
        },
    })

    local response = ReadMsg()
    local name = response.RequestResponse.response.TagName.name
    return name
end

---Set this tag's layout.
---@param layout Layout
function tg:set_layout(layout) -- TODO: output param
    tag.set_layout(self:name(), layout)
end

-----------------------------------------------------------

---Add tags.
---
---If you need to add the names as a table, use `tag.add_table` instead.
---
---### Example
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
    local tag_names = table.pack(...)
    tag_names["n"] = nil -- remove the length to make it a true array for serializing

    SendMsg({
        AddTags = {
            output_name = output.name,
            tag_names = tag_names,
        },
    })
end

---Like `tag.add`, but with a table of strings instead.
---
---### Example
---
---```lua
---local tags = { "Terminal", "Browser", "Mail", "Gaming", "Potato" }
---local output = output.get_by_name("DP-1")
---if output ~= nil then
---    tag.add(output, tags) -- Add tags with the names above
---end
---```
---@param output Output The output you want these tags to be added to.
---@param names string[] The names of the new tags you want to add, as a table.
function tag.add_table(output, names)
    SendMsg({
        AddTags = {
            output_name = output.name,
            tag_names = names,
        },
    })
end

---Toggle a tag on the specified output. If `output` isn't specified, toggle it on the currently focused output instead.
---
---### Example
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
---### Example
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

---Set a layout for the tag on the specified output. If there is none, set it for the tag on the currently focused one.
---@param name string The name of the tag.
---@param layout Layout The layout.
---@param output Output? The output.
function tag.set_layout(name, layout, output)
    if output ~= nil then
        SendMsg({
            SetLayout = {
                output_name = output.name,
                tag_name = name,
                layout = layout,
            },
        })
    else
        local op = require("output").get_focused()
        if op ~= nil then
            SendMsg({
                SetLayout = {
                    output_name = op.name,
                    tag_name = name,
                    layout = layout,
                },
            })
        end
    end
end

---Get all tags on the specified output.
---
---You can also use `output_obj:tags()`, which delegates to this function:
---```lua
---local tags_on_output = output.get_focused():tags()
----- This is the same as
----- local tags_on_output = tag.get_on_output(output.get_focused())
---```
---@param output Output
---@return Tag[]
function tag.get_on_output(output)
    SendRequest({
        GetTagsByOutput = {
            output_name = output.name,
        },
    })

    local response = ReadMsg()

    local tag_ids = response.RequestResponse.response.Tags.tag_ids

    ---@type Tag[]
    local tags = {}

    for _, tag_id in pairs(tag_ids) do
        table.insert(tags, new_tag({ id = tag_id }))
    end

    return tags
end

return tag
