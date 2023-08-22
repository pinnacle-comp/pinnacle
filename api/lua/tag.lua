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

---@alias TagTable { [1]: string, [2]: (string|Output)? }
---@alias TagTableNamed { name: string, output: (string|Output)? }

---@class Tag
---@field private _id integer The internal id of this tag.
local tag = {}

---Create a tag from `Tag|TagTable|TagTableNamed|string`.
---@param tb Tag|TagTable|TagTableNamed|string
---@return Tag|nil
local function create_tag_from_params(tb)
    -- If creating from a tag object, just return the obj
    if tb.id then
        return tb --[[@as Tag]]
    end

    -- string passed in
    if type(tb) == "string" then
        local op = require("output").get_focused()
        if op == nil then
            return nil
        end

        local tags = tag_module.get_by_name(tb)
        for _, t in pairs(tags) do
            if t:output() and t:output():name() == op:name() then
                return t
            end
        end

        return nil
    end

    -- TagTable was passed in
    local tag_name = tb[1]
    if type(tag_name) == "string" then
        local op = tb[2]
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

    -- TagTableNamed was passed in
    local tb = tb --[[@as TagTableNamed]]
    local tag_name = tb.name
    local op = tb.output

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
----- Verbose versions of the two above
---tag.toggle({ name = "1", output = "DP-1" })
---tag.toggle({ name = "1", output = op })
---
----- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.toggle(t)
---```
---@param t Tag|TagTable|TagTableNamed|string
---@see Tag.toggle — The corresponding object method
function tag_module.toggle(t)
    local t = create_tag_from_params(t)

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
----- Verbose versions of the two above
---tag.switch_to({ name = "1", output = "DP-1" })
---tag.switch_to({ name = "1", output = op })
---
----- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.switch_to(t)
---```
---@param t Tag|TagTable|TagTableNamed|string
---@see Tag.switch_to — The corresponding object method
function tag_module.switch_to(t)
    local t = create_tag_from_params(t)

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
---tag.set_layout({ "1" }, "Dwindle") -- Same as above
---
---tag.set_layout({ "1", "DP-1" }, "Dwindle") -- Set tag 1 on DP-1 to "Dwindle"
---tag.set_layout({ "1", op }, "Dwindle")     -- Same as above
---
----- Verbose versions of the two above
---tag.set_layout({ name = "1", output = "DP-1" }, "Dwindle")
---tag.set_layout({ name = "1", output = op }, "Dwindle")
---
----- Using a tag object
---local t = tag.get_by_name("1")[1] -- `t` is the first tag with the name "1"
---tag.set_layout(t, "Dwindle")
---```
---
---@param t Tag|TagTable|TagTableNamed|string
---@param layout Layout The layout.
---@see Tag.set_layout — The corresponding object method
function tag_module.set_layout(t, layout)
    local t = create_tag_from_params(t)

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
---local t = tag.get({ "1", "HDMI-A-0" })
---local t = tag.get({ name = "3" })
---
---local op = output.get_by_name("DP-2")
---if op ~= nil then
---    local t = tag.get({ name = "Code", output = op })
---end
---```
---@param params TagTable|TagTableNamed|string
---
---@see TagModule.get_on_output
---@see TagModule.get_by_name
---@see TagModule.get_all
function tag_module.get(params)
    return create_tag_from_params(params)
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
