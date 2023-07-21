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
---@field private _id integer The internal id of this tag.
local tg = {}

---@param tag_id integer
---@return Tag
local function new_tag(tag_id)
    ---@type Tag
    local t = { _id = tag_id }
    -- Copy functions over
    for k, v in pairs(tg) do
        t[k] = v
    end

    return t
end

---Switch to this tag.
function tg:switch_to()
    tag.switch_to(self)
end

---Toggle this tag.
function tg:toggle()
    tag.toggle(self)
end

---Get this tag's internal id.
---@return integer
function tg:id()
    return self._id
end

---Get this tag's active status.
---@return boolean|nil active `true` if the tag is active, `false` if not, and `nil` if the tag doesn't exist.
function tg:active()
    SendRequest({
        GetTagProps = {
            tag_id = self._id,
        },
    })

    local response = ReadMsg()
    local active = response.RequestResponse.response.TagProps.active
    return active
end

---Get this tag's name.
---@return string|nil name The name of this tag, or nil if it doesn't exist.
function tg:name()
    SendRequest({
        GetTagProps = {
            tag_id = self._id,
        },
    })

    local response = ReadMsg()
    local name = response.RequestResponse.response.TagProps.name
    return name
end

---Get this tag's output.
---@return Output|nil output The output this tag is on, or nil if the tag doesn't exist.
function tg:output()
    return require("output").get_for_tag(self)
end

---Set this tag's layout.
---@param layout Layout
function tg:set_layout(layout)
    tag.set_layout(self, layout)
end

-----------------------------------------------------------

---Add tags to the specified output.
---
---You can also do `output_object:add_tags(...)`.
---
---### Examples
---
---```lua
---local op = output.get_by_name("DP-1")
---if op ~= nil then
---    tag.add(op, "1", "2", "3", "4", "5") -- Add tags with names 1-5
---end
---```
---You can also pass in a table.
---```lua
---local tags = {"Terminal", "Browser", "Code", "Potato", "Email"}
---tag.add(op, tags) -- Add tags with those names
---```
---@param output Output The output you want these tags to be added to.
---@param ... string The names of the new tags you want to add.
---@overload fun(output: Output, tag_names: string[])
function tag.add(output, ...)
    local varargs = { ... }
    if type(varargs[1]) == "string" then
        local tag_names = varargs
        tag_names["n"] = nil -- remove the length to make it a true array for serializing

        SendMsg({
            AddTags = {
                output_name = output:name(),
                tag_names = tag_names,
            },
        })
    else
        local tag_names = varargs[1] --[=[@as string[]]=]

        SendMsg({
            AddTags = {
                output_name = output:name(),
                tag_names = tag_names,
            },
        })
    end
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
---@param t Tag
function tag.toggle(t)
    SendMsg({
        ToggleTag = {
            tag_id = t:id(),
        },
    })
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
---@param t Tag
function tag.switch_to(t)
    SendMsg({
        SwitchToTag = {
            tag_id = t:id(),
        },
    })
end

---Set a layout for the specified tag.
---@param t Tag
---@param layout Layout
function tag.set_layout(t, layout)
    SendMsg({
        SetLayout = {
            tag_id = t:id(),
            layout = layout,
        },
    })
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
        GetOutputProps = {
            output_name = output:name(),
        },
    })

    local response = ReadMsg()

    local tag_ids = response.RequestResponse.response.OutputProps.tag_ids

    ---@type Tag[]
    local tags = {}

    if tag_ids == nil then
        return tags
    end

    for _, tag_id in pairs(tag_ids) do
        table.insert(tags, new_tag(tag_id))
    end

    return tags
end

---Get all tags with this name across all outputs.
---@param name string The name of the tags you want.
---@return Tag[]
function tag.get_by_name(name)
    SendRequest("GetTags")

    local response = ReadMsg()

    local tag_ids = response.RequestResponse.response.Tags.tag_ids

    ---@type Tag[]
    local tags = {}

    for _, tag_id in pairs(tag_ids) do
        local t = new_tag(tag_id)
        if t:name() == name then
            table.insert(tags, t)
        end
    end

    return tags
end

---Get all tags across all ouptuts.
---@return Tag[]
function tag.get_all()
    SendRequest("GetTags")

    local response = ReadMsg()

    local tag_ids = response.RequestResponse.response.Tags.tag_ids

    ---@type Tag[]
    local tags = {}

    for _, tag_id in pairs(tag_ids) do
        table.insert(tags, new_tag(tag_id))
    end

    return tags
end

return tag
