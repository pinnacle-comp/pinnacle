-- SPDX-License-Identifier: GPL-3.0-or-later

---@class OutputModule
local output_module = {}

---@class Output A display.
---@field private _name string The name of this output (or rather, of its connector).
local output = {}

---Create a new output object from a name.
---The name is the unique identifier for each output.
---@param name string
---@return Output
local function create_output(name)
    ---@type Output
    local o = { _name = name }
    -- Copy functions over
    for k, v in pairs(output) do
        o[k] = v
    end

    return o
end

---Get this output's name. This is something like "eDP-1" or "HDMI-A-0".
---@return string
function output:name()
    return self._name
end

---Get all tags on this output.
---@return Tag[]
---@see OutputModule.tags — The corresponding module function
function output:tags()
    return output_module.tags(self)
end

---Add tags to this output.
---@param ... string The names of the tags you want to add. You can also pass in a table.
---@overload fun(self: self, tag_names: string[])
---@see OutputModule.add_tags — The corresponding module function
function output:add_tags(...)
    output_module.add_tags(self, ...)
end

---Get this output's make.
---@return string|nil
---@see OutputModule.make — The corresponding module function
function output:make()
    return output_module.make(self)
end

---Get this output's model.
---@return string|nil
---@see OutputModule.model — The corresponding module function
function output:model()
    return output_module.model(self)
end

---Get this output's location in the global space, in pixels.
---@return { x: integer, y: integer }|nil
---@see OutputModule.loc — The corresponding module function
function output:loc()
    return output_module.loc(self)
end

---Get this output's resolution in pixels.
---@return { w: integer, h: integer }|nil
---@see OutputModule.res — The corresponding module function
function output:res()
    return output_module.res(self)
end

---Get this output's refresh rate in millihertz.
---For example, 60Hz will be returned as 60000.
---@return integer|nil
---@see OutputModule.refresh_rate — The corresponding module function
function output:refresh_rate()
    return output_module.refresh_rate(self)
end

---Get this output's physical size in millimeters.
---@return { w: integer, h: integer }|nil
---@see OutputModule.physical_size — The corresponding module function
function output:physical_size()
    return output_module.physical_size(self)
end

---Get whether or not this output is focused. This is currently defined as having the cursor on it.
---@return boolean|nil
---@see OutputModule.focused — The corresponding module function
function output:focused()
    return output_module.focused(self)
end

---Set this output's location.
---
---### Examples
---```lua
----- Assuming DP-1 is 2560x1440 and DP-2 is 1920x1080...
---local dp1 = output.get_by_name("DP-1")
---local dp2 = output.get_by_name("DP-2")
---
----- Place DP-2 to the left of DP-1, top borders aligned
---dp1:set_loc({ x = 1920, y = 0 })
---dp2:set_loc({ x = 0, y = 0 })
---
----- Do the same as above, with a different origin
---dp1:set_loc({ x = 0, y = 0 })
---dp2:set_loc({ x = -1920, y = 0 })
---
----- Place DP-2 to the right of DP-1, bottom borders aligned
---dp1:set_loc({ x = 0, y = 0 })
---dp2:set_loc({ x = 2560, y = 1440 - 1080 })
---```
---@param loc { x: integer?, y: integer? }
function output:set_loc(loc)
    return output_module.set_loc(self, loc)
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
---print(monitor:name()) -- should print `DP-1`
---```
---@param name string The name of the output.
---@return Output|nil output The output, or nil if none have the provided name.
function output_module.get_by_name(name)
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    for _, output_name in pairs(output_names) do
        if output_name == name then
            return create_output(output_name)
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
function output_module.get_by_model(model)
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    ---@type Output[]
    local outputs = {}
    for _, output_name in pairs(output_names) do
        local o = create_output(output_name)
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
function output_module.get_by_res(width, height)
    local response = Request("GetOutputs")

    local output_names = response.RequestResponse.response.Outputs.output_names

    ---@type Output[]
    local outputs = {}
    for _, output_name in pairs(output_names) do
        local o = create_output(output_name)
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
function output_module.get_focused()
    local response = Request("GetOutputs")
    local output_names = response.RequestResponse.response.Outputs.output_names

    for _, output_name in pairs(output_names) do
        local o = create_output(output_name)
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
function output_module.connect_for_all(func)
    ---@param args Args
    table.insert(CallbackTable, function(args)
        local args = args.ConnectForAllOutputs
        func(create_output(args.output_name))
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
---@see TagModule.output — A global method for fully qualified syntax (for you Rustaceans out there)
---@see Tag.output — The corresponding object method
function output_module.get_for_tag(tag)
    local response = Request({
        GetTagProps = {
            tag_id = tag:id(),
        },
    })
    local output_name = response.RequestResponse.response.TagProps.output_name

    if output_name == nil then
        return nil
    else
        return create_output(output_name)
    end
end

---------Fully-qualified functions

---Get the specified output's make.
---@param op Output
---@return string|nil
---@see Output.make — The corresponding object method
function output_module.make(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.make
end

---Get the specified output's model.
---@param op Output
---@return string|nil
---@see Output.model — The corresponding object method
function output_module.model(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.model
end

---Get the specified output's location in the global space, in pixels.
---@param op Output
---@return { x: integer, y: integer }|nil
---@see Output.loc — The corresponding object method
function output_module.loc(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.loc == nil then
        return nil
    else
        return { x = props.loc[1], y = props.loc[2] }
    end
end

---Get the specified output's resolution in pixels.
---@param op Output
---@return { w: integer, h: integer }|nil
---@see Output.res — The corresponding object method
function output_module.res(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.res == nil then
        return nil
    else
        return { w = props.res[1], h = props.res[2] }
    end
end

---Get the specified output's refresh rate in millihertz.
---For example, 60Hz will be returned as 60000.
---@param op Output
---@return integer|nil
---@see Output.refresh_rate — The corresponding object method
function output_module.refresh_rate(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.refresh_rate
end

---Get the specified output's physical size in millimeters.
---@param op Output
---@return { w: integer, h: integer }|nil
---@see Output.physical_size — The corresponding object method
function output_module.physical_size(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    if props.physical_size == nil then
        return nil
    else
        return { w = props.physical_size[1], h = props.physical_size[2] }
    end
end

---Get whether or not the specified output is focused. This is currently defined as having the cursor on it.
---@param op Output
---@return boolean|nil
---@see Output.focused — The corresponding object method
function output_module.focused(op)
    local response = Request({
        GetOutputProps = {
            output_name = op:name(),
        },
    })
    local props = response.RequestResponse.response.OutputProps
    return props.focused
end

---Get the specified output's tags.
---@param op Output
---@see TagModule.get_on_output — The called function
---@see Output.tags — The corresponding object method
function output_module.tags(op)
    return require("tag").get_on_output(op)
end

---Add tags to the specified output.
---@param op Output
---@param ... string The names of the tags you want to add. You can also pass in a table.
---@overload fun(op: Output, tag_names: string[])
---@see TagModule.add — The called function
---@see Output.add_tags — The corresponding object method
function output_module.add_tags(op, ...)
    require("tag").add(op, ...)
end

---Set the specified output's location.
---
---### Examples
---```lua
----- Assuming DP-1 is 2560x1440 and DP-2 is 1920x1080...
---local dp1 = output.get_by_name("DP-1")
---local dp2 = output.get_by_name("DP-2")
---
----- Place DP-2 to the left of DP-1, top borders aligned
---output.set_loc(dp1, { x = 1920, y = 0 })
---output.set_loc(dp2, { x = 0, y = 0 })
---
----- Do the same as above, with a different origin
---output.set_loc(dp1, { x = 0, y = 0 })
---output.set_loc(dp2, { x = -1920, y = 0 })
---
----- Place DP-2 to the right of DP-1, bottom borders aligned
---output.set_loc(dp1, { x = 0, y = 0 })
---output.set_loc(dp2, { x = 2560, y = 1440 - 1080 })
---```
---@param op Output
---@param loc { x: integer?, y: integer? }
function output_module.set_loc(op, loc)
    SendMsg({
        SetOutputLocation = {
            output_name = op:name(),
            x = loc.x,
            y = loc.y,
        },
    })
end

return output_module
