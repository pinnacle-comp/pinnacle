-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@class OutputGlobal
local output_global = {}

---@class Output A display.
---@field private _name string The name of this output (or rather, of its connector).
local output = {}

---Get this output's name. This is something like "eDP-1" or "HDMI-A-0".
---@return string
function output:name()
    return self._name
end

---Get all tags on this output. See `tag.get_on_output`.
---@return Tag[]
function output:tags()
    return require("tag").get_on_output(self)
end

---Add tags to this output. See `tag.add`.
---@param ... string The names of the tags you want to add.
---@overload fun(self: self, tag_names: string[])
function output:add_tags(...)
    require("tag").add(self, ...)
end

---Get this output's make.
---@return string|nil
function output:make()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.make
end

---Get this output's model.
---@return string|nil
function output:model()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.model
end

---Get this output's location in the global space.
---@return { x: integer, y: integer }|nil
function output:loc()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.loc == nil then
        return nil
    else
        return { x = props.loc[1], y = props.loc[2] }
    end
end

---Get this output's resolution in pixels.
---@return { w: integer, h: integer }|nil
function output:res()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.res == nil then
        return nil
    else
        return { w = props.res[1], h = props.res[2] }
    end
end

---Get this output's refresh rate in millihertz.
---For example, 60Hz will be returned as 60000.
---@return integer|nil
function output:refresh_rate()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.refresh_rate
end

---Get this output's physical size in millimeters.
---@return { w: integer, h: integer }|nil
function output:physical_size()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.physical_size == nil then
        return nil
    else
        return { w = props.physical_size[1], h = props.physical_size[2] }
    end
end

---Get whether or not this output is focused. This is currently defined as having the cursor on it.
---@return boolean|nil
function output:focused()
    local response = Request({
        GetOutputProps = {
            output_name = self._name,
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.focused
end

---This is an internal global function used to create an output object from an output name.
---@param output_name string The name of the output.
---@return Output
local function new_output(output_name)
    ---@type Output
    local o = { _name = output_name }
    -- Copy functions over
    for k, v in pairs(output) do
        o[k] = v
    end

    return o
end

------------------------------------------------------

---Get an output by its name.
---
---"Name" in this sense does not mean its model or manufacturer;
---rather, "name" is the name of the connector the output is connected to.
---This should be something like "HDMI-A-0", "eDP-1", or similar.
---
---### Example
---```lua
---local monitor = output.get_by_name("DP-1")
---print(monitor.name) -- should print `DP-1`
---```
---@param name string The name of the output.
---@return Output|nil output The output, or nil if none have the provided name.
function output_global.get_by_name(name)
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    for _, output_name in pairs(output_names) do
        if output_name == name then
            return new_output(output_name)
        end
    end

    return nil
end

---Note: This may or may not be what is reported by other monitor listing utilities. Pinnacle currently fails to pick up one of my monitors' models when it is correctly picked up by tools like wlr-randr. I'll fix this in the future.
---
---Get outputs by their model.
---This is something like "DELL E2416H" or whatever gibberish monitor manufacturers call their displays.
---@param model string The model of the output(s).
---@return Output[] outputs All outputs with this model.
function output_global.get_by_model(model)
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    ---@type Output[]
    local outputs = {}
    for _, output_name in pairs(output_names) do
        local o = new_output(output_name)
        if o:model() == model then
            table.insert(outputs, o)
        end
    end

    return outputs
end

---Get outputs by their resolution.
---
---@param width integer The width of the outputs, in pixels.
---@param height integer The height of the outputs, in pixels.
---@return Output[] outputs All outputs with this resolution.
function output_global.get_by_res(width, height)
    local response = Request("GetOutputs")

    local output_names = response.RequestResponse.response.Outputs.output_names

    ---@type Output
    local outputs = {}
    for _, output_name in pairs(output_names) do
        local o = new_output(output_name)
        if o:res() and o:res().w == width and o:res().h == height then
            table.insert(outputs, o)
        end
    end

    return outputs
end

---Get the currently focused output. This is currently implemented as the one with the cursor on it.
---
---This function may return nil, which means you may get a warning if you try to use it without checking for nil.
---Usually this function will not be nil unless you unplug all monitors, so instead of checking,
---you can ignore the warning by either forcing the type to be non-nil with an inline comment:
---```lua
---local op = output.get_focused() --[[@as Output]]
---```
---or by disabling nil check warnings for the line:
---```lua
---local op = output.get_focused()
------@diagnostic disable-next-line:need-check-nil
---local tags_on_output = op:tags()
---```
---Type checking done by Lua LS isn't perfect.
---Note that directly using the result of this function inline will *not* raise a warning, so be careful.
---```lua
---local tags = output.get_focused():tags() -- will NOT warn for nil
---```
---@return Output|nil output The output, or nil if none are focused.
function output_global.get_focused()
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    for _, output_name in pairs(output_names) do
        local o = new_output(output_name)
        if o:focused() then
            return o
        end
    end

    return nil
end

---Connect a function to be run on all current and future outputs.
---
---When called, `connect_for_all` will immediately run `func` with all currently connected outputs.
---If a new output is connected, `func` will also be called with it.
---
---Please note: this function will be run *after* Pinnacle processes your entire config.
---For example, if you define tags in `func` but toggle them directly after `connect_for_all`, nothing will happen as the tags haven't been added yet.
---@param func fun(output: Output) The function that will be run.
function output_global.connect_for_all(func)
    ---@param args Args
    table.insert(CallbackTable, function(args)
        local args = args.ConnectForAllOutputs
        func(new_output(args.output_name))
    end)
    SendMsg({
        ConnectForAllOutputs = {
            callback_id = #CallbackTable,
        },
    })
end

---Get the output the specified tag is on.
---@param tag Tag
---@return Output|nil
function output_global.get_for_tag(tag)
    local response = Request({
        GetTagProps = {
            tag_id = tag:id(),
        },
    })
    local output_name = response.RequestResponse.response.TagProps.output_name

    if output_name == nil then
        return nil
    else
        return new_output(output_name)
    end
end

return output_global
