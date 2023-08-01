-- SPDX-License-Identifier: GPL-3.0-or-later

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
---```
---You can also pass in a table.
---```lua
---local tags = {"Terminal", "Browser", "Code", "Potato", "Email"}
---tag.add(op, tags) -- Add tags with those names
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
---@overload fun(t: Tag)
---@see Tag.toggle — The corresponding object method
function tag_module.toggle(name, output)
    if type(name) == "table" then
        SendMsg({
            ToggleTag = {
                tag_id = name--[[@as Tag]]:id(),
            },
        })
        return
    end

    local output = output or require("output").get_focused()

    if output == nil then
        return
    end

    print("before tag_global.get_by_name")
    local tags = tag_module.get_by_name(name)
    print("after tag_global.get_by_name")
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == output:name() then
            SendMsg({
                ToggleTag = {
                    tag_id = t:id(),
                },
            })
            return
        end
    end
end

---Switch to a tag on the specified output, deactivating any other active tags on it.
---If `output` is not specified, this uses the currently focused output instead.
---Alternatively, provide a tag object instead of a name and output.
---
---This is used to replicate what a traditional workspace is on some other Wayland compositors.
---
---### Examples
---```lua
----- Switches to and displays *only* windows on tag `3` on the focused output.
---tag.switch_to("3")
---```
---@param name string The name of the tag.
---@param output Output? The output.
---@overload fun(t: Tag)
---@see Tag.switch_to — The corresponding object method
function tag_module.switch_to(name, output)
    if type(name) == "table" then
        SendMsg({
            SwitchToTag = {
                tag_id = name--[[@as Tag]]:id(),
            },
        })
        return
    end

    local output = output or require("output").get_focused()

    if output == nil then
        return
    end

    local tags = tag_module.get_by_name(name)
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == output:name() then
            SendMsg({
                SwitchToTag = {
                    tag_id = t:id(),
                },
            })
            return
        end
    end
end

---Set a layout for the tag on the specified output. If no output is provided, set it for the tag on the currently focused one.
---Alternatively, provide a tag object instead of a name and output.
---
---### Examples
---```lua
----- Set tag `1` on `DP-1` to the `Dwindle` layout
---tag.set_layout("1", "Dwindle", output.get_by_name("DP-1"))
---
----- Do the same as above. Note: if you have more than one tag named `1` then this picks the first one.
---local t = tag.get_by_name("1")[1]
---tag.set_layout(t, "Dwindle")
---```
---@param name string The name of the tag.
---@param layout Layout The layout.
---@param output Output? The output.
---@overload fun(t: Tag, layout: Layout)
---@see Tag.set_layout — The corresponding object method
function tag_module.set_layout(name, layout, output)
    if type(name) == "table" then
        SendMsg({
            SetLayout = {
                tag_id = name--[[@as Tag]]:id(),
                layout = layout,
            },
        })
        return
    end

    local output = output or require("output").get_focused()

    if output == nil then
        return
    end

    local tags = tag_module.get_by_name(name)
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == output:name() then
            SendMsg({
                SetLayout = {
                    tag_id = t:id(),
                    layout = layout,
                },
            })
            return
        end
    end
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
----- Given one monitor with the tags "OBS", "OBS", "VSCode", and "Spotify"...
---local tags = tag.get_by_name("OBS")
----- ...will have 2 tags in `tags`, while...
---local no_tags = tag.get_by_name("Firefox")
----- ...will have `no_tags` be empty.
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
----- With two monitors with the same tags: "1", "2", "3", "4", and "5"...
---local tags = tag.get_all()
----- ...`tags` should have 10 tags, with 5 pairs of those names across both outputs.
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
----- Assuming the tag `Terminal` exists...
---print(tag.name(tag.get_by_name("Terminal")[1]))
----- ...should print `Terminal`.
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

return tag_module
