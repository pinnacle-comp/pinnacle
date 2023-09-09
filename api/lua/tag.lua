-- SPDX-License-Identifier: GPL-3.0-or-later

---Tag management.
---
---This module provides utilities for creating and manipulating tags.
---
---A tag is a sort of marker for each of your windows. It allows you to present windows in ways that
---traditional workspaces cannot.
---
---More specifically:
---
--- - A window can have multiple tags.
---     - This means that you can have one window show up across multiple "workspaces" if you come
---       something like i3.
--- - An output can display multiple tags at once.
---     - This allows you to toggle a tag and have windows on both tags display at once.
---       This is helpful if you, say, want to reference a browser window while coding; you toggle your
---       browser's tag and temporarily reference it while you work without having to change screens.
---
---Many of the functions in this module take Tag|TagTable|TagTableNamed|string.
---This is a convenience so you don't have to get a tag object every time you want to do
---something with tags.
---
---Instead, you can pass in either:
---
--- - A string of the tag's name (ex. "1")
---     - This will get the first tag with that name on the focused output.
--- - A table where [1] is the name and [2] is the output (or its name) (ex. { "1", output.get_by_name("DP-1") })
---     - This will get the first tag with that name on the specified output.
--- - The same table as above, but keyed with `name` and `output` (ex. { name = "1", output = "DP-1" })
---     - This is simply for those who want more clarity in their config.
---
---If you need to get tags beyond the first with the same name, use a `get` function and find what you need.
---@class TagModule
local tag_module = {}

---@alias Layout
---| "MasterStack" # One master window on the left with all other windows stacked to the right.
---| "Dwindle" # Windows split in half towards the bottom right corner.
---| "Spiral" # Windows split in half in a spiral.
---| "CornerTopLeft" # One main corner window in the top left with a column of windows on the right and a row on the bottom.
---| "CornerTopRight" # One main corner window in the top right with a column of windows on the left and a row on the bottom.
---| "CornerBottomLeft" # One main corner window in the bottom left with a column of windows on the right and a row on the top.
---| "CornerBottomRight" # One main corner window in the bottom right with a column of windows on the left and a row on the top.

---@alias TagTable { name: string, output: (string|Output)? }

---@alias TagConstructor Tag|TagTable|string

---A tag object.
---
---This can be retrieved through the various `get` functions in the `TagModule`.
---@classmod
---@class Tag
---@field private _id integer The internal id of this tag.
local tag = {}

---Create a tag from an id.
---The id is the unique identifier for each tag.
---@param id TagId
---@return Tag
local function create_tag(id)
    ---@type Tag
    local t = { _id = id }
    -- Copy functions over
    for k, v in pairs(tag) do
        t[k] = v
    end

    return t
end

---Get this tag's internal id.
---***You probably won't need to use this.***
---@return integer
function tag:id()
    return self._id
end

---Get this tag's active status.
---@return boolean|nil active `true` if the tag is active, `false` if not, and `nil` if the tag doesn't exist.
---@see TagModule.active — The corresponding module function
function tag:active()
    return tag_module.active(self)
end

---Get this tag's name.
---@return string|nil name The name of this tag, or nil if it doesn't exist.
---@see TagModule.name — The corresponding module function
function tag:name()
    return tag_module.name(self)
end

---Get this tag's output.
---@return Output|nil output The output this tag is on, or nil if the tag doesn't exist.
---@see TagModule.output — The corresponding module function
function tag:output()
    return tag_module.output(self)
end

---Switch to this tag.
---@see TagModule.switch_to — The corresponding module function
function tag:switch_to()
    tag_module.switch_to(self)
end

---Toggle this tag.
---@see TagModule.toggle — The corresponding module function
function tag:toggle()
    tag_module.toggle(self)
end

---Set this tag's layout.
---@param layout Layout
---@see TagModule.set_layout — The corresponding module function
function tag:set_layout(layout)
    tag_module.set_layout(self, layout)
end

-----------------------------------------------------------

---Add tags to the specified output.
---
---### Examples
---```lua
---local op = output.get_by_name("DP-1")
---if op ~= nil then
---    tag.add(op, "1", "2", "3", "4", "5") -- Add tags with names 1-5
---end
--
--- -- You can also pass in a table.
---local tags = {"Terminal", "Browser", "Code", "Potato", "Email"}
---tag.add(op, tags)
---```
---@param output Output The output you want these tags to be added to.
---@param ... string The names of the new tags you want to add.
---@overload fun(output: Output, tag_names: string[])
---@see Output.add_tags — The corresponding object method
function tag_module.add(output, ...)
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

---Toggle a tag on the specified output. If the output isn't specified, toggle it on the currently focused output instead.
---
---### Example
---
---```lua
---local op = output.get_by_name("DP-1")
---
---tag.toggle("1")             -- Toggle tag 1 on the focused output
---tag.toggle({ "1" })         -- Same as above
---
---tag.toggle({ "1", "DP-1" }) -- Toggle tag 1 on DP-1
---tag.toggle({ "1", op })     -- Same as above
---
--- -- Verbose versions of the two above
---tag.toggle({ name = "1", output = "DP-1" })
---tag.toggle({ name = "1", output = op })
---
--- -- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.toggle(t)
---```
---@param t TagConstructor
---@see Tag.toggle — The corresponding object method
function tag_module.toggle(t)
    local t = tag_module.get(t)

    if t then
        SendMsg({
            ToggleTag = {
                tag_id = t:id(),
            },
        })
    end
end

---Switch to a tag on the specified output, deactivating any other active tags on it.
---If the output is not specified, this uses the currently focused output instead.
---
---This is used to replicate what a traditional workspace is on some other Wayland compositors.
---
---### Examples
---```lua
---local op = output.get_by_name("DP-1")
---
---tag.switch_to("1")             -- Switch to tag 1 on the focused output
---tag.switch_to({ "1" })         -- Same as above
---
---tag.switch_to({ "1", "DP-1" }) -- Switch to tag 1 on DP-1
---tag.switch_to({ "1", op })     -- Same as above
---
--- -- Verbose versions of the two above
---tag.switch_to({ name = "1", output = "DP-1" })
---tag.switch_to({ name = "1", output = op })
---
--- -- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.switch_to(t)
---```
---@param t TagConstructor
---@see Tag.switch_to — The corresponding object method
function tag_module.switch_to(t)
    local t = tag_module.get(t)

    if t then
        SendMsg({
            SwitchToTag = {
                tag_id = t:id(),
            },
        })
    end
end

---Set a layout for the tag on the specified output. If no output is provided, set it for the tag on the currently focused one.
---
---### Examples
---```lua
---local op = output.get_by_name("DP-1")
---
---tag.set_layout("1", "Dwindle")     -- Set tag 1 on the focused output to "Dwindle"
---
---tag.set_layout({ name = "1", output = "DP-1" }, "Dwindle") -- Set tag 1 on "DP-1" to "Dwindle"
---tag.set_layout({ name = "1", output = op }, "Dwindle")     -- Same as above
---
--- -- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.set_layout(t, "Dwindle")
---```
---
---@param t TagConstructor
---@param layout Layout The layout.
---@see Tag.set_layout — The corresponding object method
function tag_module.set_layout(t, layout)
    local t = tag_module.get(t)

    if t then
        SendMsg({
            SetLayout = {
                tag_id = t:id(),
                layout = layout,
            },
        })
    end
end

---Get a tag with the specified name and optional output.
---
---If the output isn't specified, the focused one is used.
---
---If you have duplicate tags on an output, this returns the first one.
---If you need access to all duplicates, use `tag.get_on_output`, `tag.get_by_name`, or `tag.get_all`
---and filter for what you need.
---
---### Examples
---```lua
---local t = tag.get("1")
---local t = tag.get({ name = "3" })
---local t = tag.get({ name = "1", output = "HDMI-A-0" })
---
---local op = output.get_by_name("DP-2")
---if op ~= nil then
---    local t = tag.get({ name = "Code", output = op })
---end
---```
---@param params TagConstructor
---@return Tag|nil
---
---@see TagModule.get_on_output
---@see TagModule.get_by_name
---@see TagModule.get_all
function tag_module.get(params)
    -- If creating from a tag object, just return the obj
    if params.id then
        return params --[[@as Tag]]
    end

    -- string passed in
    if type(params) == "string" then
        local op = require("output").get_focused()
        if op == nil then
            return nil
        end

        local tags = tag_module.get_by_name(params)
        for _, t in pairs(tags) do
            if t:output() and t:output():name() == op:name() then
                return t
            end
        end

        return nil
    end

    -- TagTable was passed in
    local params = params --[[@as TagTable]]
    local tag_name = params.name
    local op = params.output

    if op == nil then
        local o = require("output").get_focused()
        if o == nil then
            return nil
        end
        op = o
    elseif type(op) == "string" then
        local o = require("output").get_by_name(op)
        if o == nil then
            return nil
        end
        op = o
    end

    local tags = tag_module.get_by_name(tag_name)
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == op:name() then
            return t
        end
    end

    return nil
end

---Get all tags on the specified output.
---
---### Example
---```lua
---local op = output.get_focused()
---if op ~= nil then
---    local tags = tag.get_on_output(op) -- All tags on the focused output
---end
---```
---@param output Output
---@return Tag[]
---
---@see Output.tags — The corresponding object method
function tag_module.get_on_output(output)
    local response = Request({
        GetOutputProps = {
            output_name = output:name(),
        },
    })

    local tag_ids = response.RequestResponse.response.OutputProps.tag_ids

    ---@type Tag[]
    local tags = {}

    if tag_ids == nil then
        return tags
    end

    for _, tag_id in pairs(tag_ids) do
        table.insert(tags, create_tag(tag_id))
    end

    return tags
end

---Get all tags with this name across all outputs.
---
---### Example
---```lua
--- -- Given one monitor with the tags "OBS", "OBS", "VSCode", and "Spotify"...
---local tags = tag.get_by_name("OBS")
--- -- ...will have 2 tags in `tags`, while...
---local no_tags = tag.get_by_name("Firefox")
--- -- ...will have `no_tags` be empty.
---```
---@param name string The name of the tag(s) you want.
---@return Tag[]
function tag_module.get_by_name(name)
    local t_s = tag_module.get_all()

    ---@type Tag[]
    local tags = {}

    for _, t in pairs(t_s) do
        if t:name() == name then
            table.insert(tags, t)
        end
    end

    return tags
end

---Get all tags across all outputs.
---
---### Example
---```lua
--- -- With two monitors with the same tags: "1", "2", "3", "4", and "5"...
---local tags = tag.get_all()
--- -- ...`tags` should have 10 tags, with 5 pairs of those names across both outputs.
---```
---@return Tag[]
function tag_module.get_all()
    local response = Request("GetTags")

    local tag_ids = response.RequestResponse.response.Tags.tag_ids

    ---@type Tag[]
    local tags = {}

    for _, tag_id in pairs(tag_ids) do
        table.insert(tags, create_tag(tag_id))
    end

    return tags
end

---Get the specified tag's name.
---
---### Example
---```lua
--- -- Assuming the tag `Terminal` exists...
---print(tag.name(tag.get_by_name("Terminal")[1]))
--- -- ...should print `Terminal`.
---```
---@param t Tag
---@return string|nil
---@see Tag.name — The corresponding object method
function tag_module.name(t)
    local response = Request({
        GetTagProps = {
            tag_id = t:id(),
        },
    })
    local name = response.RequestResponse.response.TagProps.name
    return name
end

---Get whether or not the specified tag is active.
---@param t Tag
---@return boolean|nil
---@see Tag.active — The corresponding object method
function tag_module.active(t)
    local response = Request({
        GetTagProps = {
            tag_id = t:id(),
        },
    })
    local active = response.RequestResponse.response.TagProps.active
    return active
end

---Get the output the specified tag is on.
---@param t Tag
---@return Output|nil
---@see OutputModule.get_for_tag — The called function
---@see Tag.output — The corresponding object method
function tag_module.output(t)
    return require("output").get_for_tag(t)
end

---@class LayoutCycler
---@field next fun(output: (Output|OutputName)?) Change the first active tag on `output` to its next layout. If `output` is empty, the focused output is used.
---@field prev fun(output: (Output|OutputName)?) Change the first active tag on `output` to its previous layout. If `output` is empty, the focused output is used.

---Given an array of layouts, this will create two functions; one will cycle forward the layout
---for the provided tag, and one will cycle backward.
---@param layouts Layout[] The available layouts.
---@return LayoutCycler layout_cycler A table with the functions `next` and `prev`, which will cycle layouts for the given tag.
function tag_module.layout_cycler(layouts)
    local indices = {}

    -- Return empty functions if layouts is empty
    if #layouts == 0 then
        return {
            next = function(_) end,
            prev = function(_) end,
        }
    end

    return {
        ---@param output (Output|OutputName)?
        next = function(output)
            if type(output) == "string" then
                output = require("output").get_by_name(output)
            end

            output = output or require("output").get_focused()

            if output == nil then
                return
            end

            local tags = output:tags()
            for _, tg in pairs(tags) do
                if tg:active() then
                    local id = tg:id()
                    if id == nil then
                        return
                    end

                    if #layouts == 1 then
                        indices[id] = 1
                    elseif indices[id] == nil then
                        indices[id] = 2
                    else
                        if indices[id] + 1 > #layouts then
                            indices[id] = 1
                        else
                            indices[id] = indices[id] + 1
                        end
                    end

                    tg:set_layout(layouts[indices[id]])
                    break
                end
            end
        end,

        ---@param output (Output|OutputName)?
        prev = function(output)
            if type(output) == "string" then
                output = require("output").get_by_name(output)
            end

            output = output or require("output").get_focused()

            if output == nil then
                return
            end

            local tags = output:tags()
            for _, tg in pairs(tags) do
                if tg:active() then
                    local id = tg:id()
                    if id == nil then
                        return
                    end

                    if #layouts == 1 then
                        indices[id] = 1
                    elseif indices[id] == nil then
                        indices[id] = #layouts - 1
                    else
                        if indices[id] - 1 < 1 then
                            indices[id] = #layouts
                        else
                            indices[id] = indices[id] - 1
                        end
                    end

                    tg:set_layout(layouts[indices[id]])
                    break
                end
            end
        end,
    }
end

return tag_module
