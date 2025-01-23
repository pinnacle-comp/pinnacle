-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local tag_service = require("pinnacle.grpc.defs").pinnacle.tag.v1.TagService

local set_or_toggle = {
    SET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    [true] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    UNSET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    [false] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    TOGGLE = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_TOGGLE,
}

---@lcat nodoc
---@class TagHandleModule
local tag_handle = {}

---A tag handle.
---
---This is a handle that allows manipulation of a tag.
---
---This can be retrieved through the various `get` functions in the `Tag` module.
---@classmod
---@class TagHandle
---@field id integer
local TagHandle = {}

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
---   - This means that you can have one window show up across multiple "workspaces" if you come something like i3.
--- - An output can display multiple tags at once.
---   - This allows you to toggle a tag and have windows on both tags display at once. This is helpful if you, say, want to reference a browser window while coding; you toggle your browser's tag and temporarily reference it while you work without having to change screens.
---
---If you need to get tags beyond the first with the same name, use the `get` method and find what you need.
---@class Tag
---@field private handle TagHandleModule
local tag = {}
tag.handle = tag_handle

---Gets all tags across all outputs.
---
---@return TagHandle[]
function tag.get_all()
    local response, err = client:unary_request(tag_service.Get, {})

    if err then
        log:error(err)
        return {}
    end

    ---@cast response pinnacle.tag.v1.GetResponse

    ---@type TagHandle[]
    local handles = {}

    for _, id in ipairs(response.tag_ids or {}) do
        table.insert(handles, tag_handle.new(id))
    end

    return handles
end

---Gets the tag with the given name and output.
---
---If `output` is not specified, this uses the focused output.
---
---If an output has more than one tag with the same name, this returns the first.
---
---#### Example
---```lua
--- -- Get tags on the focused output
---local tag = Tag.get("Tag")
---
--- -- Get tags on a specific output
---local tag_on_hdmi1 = Tag.get("Tag", Output.get_by_name("HDMI-1"))
---```
---
---@param name string
---@param output OutputHandle?
---
---@return TagHandle | nil
function tag.get(name, output)
    output = output or require("pinnacle.output").get_focused()

    if not output then
        return
    end

    local handles = tag.get_all()

    ---@type (fun(): { output: OutputHandle, name: string })[]
    local requests = {}

    for i, handle in ipairs(handles) do
        requests[i] = function()
            return {
                output = handle:output(),
                name = handle:name(),
            }
        end
    end

    local props = require("pinnacle.util").batch(requests)

    for i, prop in ipairs(props) do
        if prop.output.name == output.name and prop.name == name then
            return handles[i]
        end
    end

    return nil
end

---Adds tags with the given names to the specified output.
---
---Returns handles to the created tags.
---
---#### Example
---```lua
---local tags = Tag.add(Output.get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
--- -- With a table
---local tag_names = { "1", "2", "Buckle", "Shoe" }
---local tags = Tag.add(Output.get_by_name("HDMI-1"), tag_names)
---```
---
---@param output OutputHandle
---@param ... string
---
---@return TagHandle[] tags Handles to the created tags
---
---@overload fun(output: OutputHandle, tag_names: string[]): TagHandle[]
function tag.add(output, ...)
    local tag_names = { ... }
    if type(tag_names[1]) == "table" then
        tag_names = tag_names[1] --[=[@as string[]]=]
    end

    local response, err = client:unary_request(tag_service.Add, {
        output_name = output.name,
        tag_names = tag_names,
    })

    if err then
        log:error(err)
        return {}
    end

    ---@cast response pinnacle.tag.v1.AddResponse

    ---@type TagHandle[]
    local handles = {}

    for _, id in ipairs(response.tag_ids or {}) do
        table.insert(handles, tag_handle.new(id))
    end

    return handles
end

---Removes the given tags.
---
---#### Example
---```lua
---local tags = Tag.add(Output.get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
---Tag.remove(tags) -- "HDMI-1" no longer has those tags
---```
---
---@param tags TagHandle[]
function tag.remove(tags)
    ---@type integer[]
    local ids = {}

    for _, tg in ipairs(tags) do
        table.insert(ids, tg.id)
    end

    local _, err = client:unary_request(tag_service.Remove, { tag_ids = ids })

    if err then
        log:error(err)
    end
end

local signal_name_to_SignalName = {
    active = "TagActive",
}

---@class TagSignal Signals related to tag events.
---@field active fun(tag: TagHandle, active: boolean)? A tag was set to active or not active.

---Connects to a tag signal.
---
---`signals` is a table containing the signal(s) you want to connect to along with
---a corresponding callback that will be called when the signal is signalled.
---
---This function returns a table of signal handles with each handle stored at the same key used
---to connect to the signal. See `SignalHandles` for more information.
---
---# Example
---```lua
---Tag.connect_signal({
---    active = function(tag, active)
---        print("Activity for " .. tag:name() .. " was set to", active)
---    end
---})
---```
---
---@param signals TagSignal The signal you want to connect to
---
---@return SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see SignalHandles.disconnect_all - To disconnect from these signals
function tag.connect_signal(signals)
    ---@diagnostic disable-next-line: invisible
    local handles = require("pinnacle.signal").handles.new({})

    for signal, callback in pairs(signals) do
        require("pinnacle.signal").add_callback(signal_name_to_SignalName[signal], callback)
        local handle =
            ---@diagnostic disable-next-line: invisible
            require("pinnacle.signal").handle.new(signal_name_to_SignalName[signal], callback)
        handles[signal] = handle
    end

    return handles
end

--------------------------------------------------------------

---Removes this tag.
---
---#### Example
---```lua
---local tags = Tag.add(Output.get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
---tags[2]:remove()
---tags[4]:remove()
--- -- "HDMI-1" now only has tags "1" and "Buckle"
---```
function TagHandle:remove()
    tag.remove({ self })
end

---Activates this tag and deactivates all other ones on the same output.
---
---#### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag.get("2"):switch_to() -- Displays Firefox and Discord
---Tag.get("3"):switch_to() -- Displays Steam
---```
function TagHandle:switch_to()
    local _, err = client:unary_request(tag_service.SwitchTo, { tag_id = self.id })

    if err then
        log:error(err)
    end
end

---Sets whether or not this tag is active.
---
---#### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag.get("2"):set_active(true)  -- Displays Firefox and Discord
---Tag.get("3"):set_active(true)  -- Displays Firefox, Discord, and Steam
---Tag.get("2"):set_active(false) -- Displays Steam
---```
---
---@param active boolean
function TagHandle:set_active(active)
    local _, err = client:unary_request(
        tag_service.SetActive,
        { tag_id = self.id, set_or_toggle = set_or_toggle[active] }
    )

    if err then
        log:error(err)
    end
end

---Toggles this tag's active state.
---
---#### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag.get("2"):toggle_active() -- Displays Firefox and Discord
---Tag.get("2"):toggle_active() -- Displays nothing
---```
function TagHandle:toggle_active()
    local _, err = client:unary_request(
        tag_service.SetActive,
        { tag_id = self.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---Gets whether or not this tag is active.
---
---@return boolean
function TagHandle:active()
    local response, err = client:unary_request(tag_service.GetActive, { tag_id = self.id })

    ---@cast response pinnacle.tag.v1.GetActiveResponse|nil

    return response and response.active or false
end

---Gets this tag's name.
---
---@return string?
function TagHandle:name()
    local response, err = client:unary_request(tag_service.GetName, { tag_id = self.id })

    ---@cast response pinnacle.tag.v1.GetNameResponse|nil

    return response and response.name or ""
end

---Gets the output this tag is on.
---
---@return OutputHandle
function TagHandle:output()
    local response, err = client:unary_request(tag_service.GetOutputName, { tag_id = self.id })

    ---@cast response pinnacle.tag.v1.GetOutputNameResponse|nil

    local output_name = response and response.output_name or ""
    return require("pinnacle.output").handle.new(output_name)
end

---Gets the windows that have this tag.
---
---@return WindowHandle[]
function TagHandle:windows()
    local windows = require("pinnacle.window").get_all()

    ---@type (fun(): TagHandle[])[]
    local win_tags = {}
    for i, window in ipairs(windows) do
        win_tags[i] = function()
            return window:tags()
        end
    end

    local tags = require("pinnacle.util").batch(win_tags)
    local wins_on_tag = {}
    for i, tags in ipairs(tags) do
        for _, tag in ipairs(tags) do
            if tag.id == self.id then
                table.insert(wins_on_tag, windows[i])
                break
            end
        end
    end

    return wins_on_tag
end

---Creates a new `TagHandle` from an id.
---@param tag_id integer
---@return TagHandle
function tag_handle.new(tag_id)
    ---@type TagHandle
    local self = {
        id = tag_id,
    }
    setmetatable(self, { __index = TagHandle })
    return self
end

---@param tag_ids integer[]
---@return TagHandle[]
function tag_handle.new_from_table(tag_ids)
    ---@type TagHandle[]
    local handles = {}

    for _, id in ipairs(tag_ids) do
        table.insert(handles, tag_handle.new(id))
    end

    return handles
end

return tag
