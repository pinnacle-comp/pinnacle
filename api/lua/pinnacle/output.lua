-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")

---The protobuf absolute path prefix
local prefix = "pinnacle.output." .. client.version .. "."
local service = prefix .. "OutputService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) OutputServiceMethod
local rpc_types = {
    SetLocation = {},
    SetMode = {},
    SetScale = {},
    ConnectForAll = {
        response_type = "ConnectForAllResponse",
    },
    Get = {
        response_type = "GetResponse",
    },
    GetProperties = {
        response_type = "GetPropertiesResponse",
    },
}

---Build GrpcRequestParams
---@param method OutputServiceMethod
---@param data table
---@return GrpcRequestParams
local function build_grpc_request_params(method, data)
    local req_type = rpc_types[method].request_type
    local resp_type = rpc_types[method].response_type

    ---@type GrpcRequestParams
    return {
        service = service,
        method = method,
        request_type = req_type and prefix .. req_type or prefix .. method .. "Request",
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---@nodoc
---@class OutputHandleModule
local output_handle = {}

---An output handle.
---
---This is a handle to one of your monitors.
---It serves to make it easier to deal with them, defining methods for getting properties and
---helpers for things like positioning multiple monitors.
---
---This can be retrieved through the various `get` functions in the `Output` module.
---@classmod
---@class OutputHandle
---@field name string The unique name of this output
local OutputHandle = {}

---Output management.
---
---An output is what you would call a monitor. It presents windows, your cursor, and other UI elements.
---
---Outputs are uniquely identified by their name, a.k.a. the name of the connector they're plugged in to.
---@class Output
---@field private handle OutputHandleModule
local output = {}
output.handle = output_handle

---Get all outputs.
---
---### Example
---```lua
---local outputs = Output.get_all()
---```
---
---@return OutputHandle[]
function output.get_all()
    -- Not going to batch these because I doubt people would have that many monitors

    local response = client.unary_request(build_grpc_request_params("Get", {}))

    ---@type OutputHandle[]
    local handles = {}

    for _, output_name in ipairs(response.output_names or {}) do
        table.insert(handles, output_handle.new(output_name))
    end

    return handles
end

---Get an output by its name (the connector it's plugged into).
---
---### Example
---```lua
---local output = Output.get_by_name("eDP-1")
---```
---
---@param name string The name of the connector the output is connected to
---@return OutputHandle | nil
function output.get_by_name(name)
    local handles = output.get_all()

    for _, handle in ipairs(handles) do
        if handle.name == name then
            return handle
        end
    end

    return nil
end

---Get the currently focused output.
---
---This is currently defined as the most recent one that has had pointer motion.
---
---### Example
---```lua
---local output = Output.get_focused()
---```
---
---@return OutputHandle | nil
function output.get_focused()
    local handles = output.get_all()

    for _, handle in ipairs(handles) do
        if handle:props().focused then
            return handle
        end
    end

    return nil
end

---Connect a function to be run with all current and future outputs.
---
---This method does two things:
---1. Immediately runs `callback` with all currently connected outputs.
---2. Calls `callback` whenever a new output is plugged in.
---
---This will *not* run `callback` with an output that has been unplugged and replugged
---to prevent duplicate setup. Instead, the compositor keeps track of the tags and other
---state associated with that output and restores it when replugged.
---
---### Example
---```lua
--- -- Add tags "1" through "5" to all outputs
---Output.connect_for_all(function(output)
---    local tags = Tag.add(output, "1", "2", "3", "4", "5")
---    tags[1]:toggle_active()
---end)
---```
---
---@param callback fun(output: OutputHandle)
function output.connect_for_all(callback)
    local handles = output.get_all()
    for _, handle in ipairs(handles) do
        callback(handle)
    end

    output.connect_signal({
        connect = callback,
    })
end

---@class OutputSetupArgs
---@field [1] (string | fun(output: OutputHandle): boolean)
---@field loc ({ x: integer, y: integer } | { [1]: (string | fun(output: OutputHandle): boolean), [2]: Alignment })?
---@field mode Mode?
---@field scale number?
---@field tag_names string[]?

---comment
---@param op OutputHandle
---@param matcher string | fun(output: OutputHandle): boolean
---@return boolean
local function output_matches(op, matcher)
    return (type(matcher) == "string" and matcher == op.name) or (type(matcher) == "function" and matcher(op))
end

---Declaratively setup outputs.
---
---`Output.setup` allows you to specify output properties that will be applied immediately and
---on output connection. These include location, mode, scale, and tags.
---
---Arguments will be applied from top to bottom.
---
---`loc` will not be applied to arguments with an output matching function.
---
---@param setup OutputSetupArgs[]
function output.setup(setup)
    ---@param op OutputHandle
    local function apply_transformers(op)
        for _, args in ipairs(setup) do
            if output_matches(op, args[1]) then
                if args.mode then
                    op:set_mode(args.mode.pixel_width, args.mode.pixel_height, args.mode.refresh_rate_millihz)
                end
                if args.scale then
                    op:set_scale(args.scale)
                end
                if args.tag_names then
                    local tags = require("pinnacle.tag").add(op, args.tag_names)
                    tags[1]:set_active(true)
                end
            end
        end
    end

    -- Apply mode, scale, and transforms first
    local outputs = output.get_all()

    for _, op in ipairs(outputs) do
        apply_transformers(op)
    end

    local function layout_outputs()
        local outputs = output.get_all()

        ---@type table<string, { x: integer, y: integer }>
        local placed_outputs = {}

        local rightmost_output = {
            output = nil,
            x = nil,
        }

        local relative_to_outputs_that_are_not_placed = {}

        -- Place outputs with a specified location first
        for _, args in ipairs(setup) do
            for _, op in ipairs(outputs) do
                if type(args[1]) == "string" and op.name == args[1] then
                    if args.loc and args.loc.x and args.loc.y then
                        local loc = { x = args.loc.x, y = args.loc.y }
                        op:set_location(loc)
                        placed_outputs[op.name] = loc

                        local props = op:props()
                        if not rightmost_output.x or rightmost_output.x < props.x + props.logical_width then
                            rightmost_output.output = op
                            rightmost_output.x = props.x + props.logical_width
                        end
                    end
                end
            end
        end

        -- Place outputs with no specified location in a line to the rightmost
        for _, op in ipairs(outputs) do
            local args_contains_op = false

            for _, args in ipairs(setup) do
                if type(args[1]) == "string" and op.name == args[1] then
                    args_contains_op = true
                    if not args.loc then
                        if not rightmost_output.output then
                            op:set_location({ x = 0, y = 0 })
                        else
                            op:set_loc_adj_to(rightmost_output.output, "right_align_top")
                        end

                        local props = op:props()

                        local loc = { x = props.x, y = props.y }
                        rightmost_output.output = op
                        rightmost_output.x = props.x

                        placed_outputs[op.name] = loc

                        goto continue_outer
                    end
                end
            end

            -- No match, still lay it out

            if not args_contains_op and not placed_outputs[op.name] then
                if not rightmost_output.output then
                    op:set_location({ x = 0, y = 0 })
                else
                    op:set_loc_adj_to(rightmost_output.output, "right_align_top")
                end

                local props = op:props()

                local loc = { x = props.x, y = props.y }
                rightmost_output.output = op
                rightmost_output.x = props.x

                placed_outputs[op.name] = loc
            end

            ::continue_outer::
        end

        -- Place outputs that are relative to other outputs
        for _, args in ipairs(setup) do
            for _, op in ipairs(outputs) do
                if type(args[1]) == "string" and op.name == args[1] then
                    if args.loc and args.loc[1] and args.loc[2] then
                        local matcher = args.loc[1]
                        local alignment = args.loc[2]
                        ---@type OutputHandle?
                        local relative_to = nil

                        for _, op in ipairs(outputs) do
                            if output_matches(op, matcher) then
                                relative_to = op
                                break
                            end
                        end

                        if not relative_to then
                            table.insert(relative_to_outputs_that_are_not_placed, op)
                            goto continue
                        end

                        if not placed_outputs[relative_to.name] then
                            -- The output it's relative to hasn't been placed yet;
                            -- Users must place outputs before ones being placed relative to them
                            table.insert(relative_to_outputs_that_are_not_placed, op)
                            goto continue
                        end

                        op:set_loc_adj_to(relative_to, alignment)

                        local props = op:props()

                        local loc = { x = props.x, y = props.y }

                        if not rightmost_output.output or rightmost_output.x < props.x + props.logical_width then
                            rightmost_output.output = op
                            rightmost_output.x = props.x + props.logical_width
                        end

                        placed_outputs[op.name] = loc
                    end
                end
                ::continue::
            end
        end

        -- Place still-not-placed outputs
        for _, op in ipairs(relative_to_outputs_that_are_not_placed) do
            if not rightmost_output.output then
                op:set_location({ x = 0, y = 0 })
            else
                op:set_loc_adj_to(rightmost_output.output, "right_align_top")
            end

            local props = op:props()

            local loc = { x = props.x, y = props.y }
            rightmost_output.output = op
            rightmost_output.x = props.x

            placed_outputs[op.name] = loc
        end
    end

    layout_outputs()

    output.connect_signal({
        connect = function(op)
            -- FIXME: This currently does not duplicate tags because the connect signal does not fire for previously connected
            -- |      outputs. However, this is unintended behavior, so fix this when you fix that.
            apply_transformers(op)
            layout_outputs()
        end,
        disconnect = function(_)
            layout_outputs()
        end,
        resize = function(_, _, _)
            layout_outputs()
        end,
    })
end

---@type table<string, SignalServiceMethod>
local signal_name_to_SignalName = {
    connect = "OutputConnect",
    disconnect = "OutputDisconnect",
    resize = "OutputResize",
    move = "OutputMove",
}

---@class OutputSignal Signals related to output events.
---@field connect fun(output: OutputHandle)? An output was connected. FIXME: This currently does not fire for outputs that have been previously connected and disconnected.
---@field disconnect fun(output: OutputHandle)? An output was disconnected.
---@field resize fun(output: OutputHandle, logical_width: integer, logical_height: integer)? An output's logical size changed.
---@field move fun(output: OutputHandle, x: integer, y: integer)? An output moved.

---Connect to an output signal.
---
---The compositor sends signals about various events. Use this function to run a callback when
---some output signal occurs.
---
---This function returns a table of signal handles with each handle stored at the same key used
---to connect to the signal. See `SignalHandles` for more information.
---
---# Example
---```lua
---Output.connect_signal({
---    connect = function(output)
---        print("New output connected:", output.name)
---    end
---})
---```
---
---@param signals OutputSignal The signal you want to connect to
---
---@return SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see SignalHandles.disconnect_all - To disconnect from these signals
function output.connect_signal(signals)
    ---@diagnostic disable-next-line: invisible
    local handles = require("pinnacle.signal").handles.new({})

    for signal, callback in pairs(signals) do
        require("pinnacle.signal").add_callback(signal_name_to_SignalName[signal], callback)
        ---@diagnostic disable-next-line: invisible
        local handle = require("pinnacle.signal").handle.new(signal_name_to_SignalName[signal], callback)
        handles[signal] = handle
    end

    return handles
end

---------------------------------------------------------------------

---Set the location of this output in the global space.
---
---On startup, Pinnacle will lay out all connected outputs starting at (0, 0)
---and going to the right, with their top borders aligned.
---
---This method allows you to move outputs where necessary.
---
---Note: If you have space between two outputs when setting their locations,
---the pointer will not be able to move between them.
---
---### Example
---```lua
--- -- Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
--- --  - "DP-1":   ┌─────┐
--- --              │     │1920x1080
--- --              └─────┘
--- --  - "HDMI-1": ┌───────┐
--- --              │ 2560x │
--- --              │ 1440  │
--- --              └───────┘
---Output.get_by_name("DP-1"):set_location({ x = 0, y = 0 })
---Output.get_by_name("HDMI-1"):set_location({ x = 1920, y = -360 })
--- -- Results in:
--- --       ┌───────┐
--- -- ┌─────┤       │
--- -- │DP-1 │HDMI-1 │
--- -- └─────┴───────┘
--- -- Notice that y = 0 aligns with the top of "DP-1", and the top of "HDMI-1" is at y = -360.
---```
---
---@param loc { x: integer?, y: integer? }
---
---@see OutputHandle.set_loc_adj_to
function OutputHandle:set_location(loc)
    client.unary_request(build_grpc_request_params("SetLocation", {
        output_name = self.name,
        x = loc.x,
        y = loc.y,
    }))
end

---@alias Alignment
---| "top_align_left" Set above, align left borders
---| "top_align_center" Set above, align centers
---| "top_align_right" Set above, align right borders
---| "bottom_align_left" Set below, align left borders
---| "bottom_align_center" Set below, align centers
---| "bottom_align_right" Set below, align right border
---| "left_align_top" Set to left, align top borders
---| "left_align_center" Set to left, align centers
---| "left_align_bottom" Set to left, align bottom borders
---| "right_align_top" Set to right, align top borders
---| "right_align_center" Set to right, align centers
---| "right_align_bottom" Set to right, align bottom borders

---Set the location of this output adjacent to another one.
---
---`alignment` is how you want this output to be placed.
---For example, "top_align_left" will place this output above `other` and align the left borders.
---Similarly, "right_align_center" will place this output to the right of `other` and align their centers.
---
---### Example
---```lua
--- -- Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
--- --  - "DP-1":   ┌─────┐
--- --              │     │1920x1080
--- --              └─────┘
--- --  - "HDMI-1": ┌───────┐
--- --              │ 2560x │
--- --              │ 1440  │
--- --              └───────┘
---Output.get_by_name("DP-1"):set_loc_adj_to(Output:get_by_name("HDMI-1"), "bottom_align_right")
--- -- Results in:
--- -- ┌───────┐
--- -- │       │
--- -- │HDMI-1 │
--- -- └──┬────┤
--- --    │DP-1│
--- --    └────┘
--- -- Notice that "DP-1" now has the coordinates (2280, 1440) because "DP-1" is getting moved, not "HDMI-1".
--- -- "HDMI-1" was placed at (1920, 0) during the compositor's initial output layout.
---```
---
---@param other OutputHandle
---@param alignment Alignment
function OutputHandle:set_loc_adj_to(other, alignment)
    local self_props = self:props()
    local other_props = other:props()

    if not self_props.x or not other_props.x then
        -- TODO: notify
        return
    end

    local alignment_parts = {}

    for str in alignment:gmatch("%a+") do
        table.insert(alignment_parts, str)
    end

    ---@type "top"|"bottom"|"left"|"right"
    local dir = alignment_parts[1]
    ---@type "top"|"bottom"|"center"|"left"|"right"
    local align = alignment_parts[3]

    ---@type integer
    local x
    ---@type integer
    local y

    local self_width = self_props.logical_width
    local self_height = self_props.logical_height
    local other_width = other_props.logical_width
    local other_height = other_props.logical_height

    if not (self_width and self_height and other_width and other_height) then
        return
    end

    if dir == "top" or dir == "bottom" then
        if dir == "top" then
            y = other_props.y - self_height
        else
            y = other_props.y + other_height
        end

        if align == "left" then
            x = other_props.x
        elseif align == "center" then
            x = other_props.x + math.floor((other_width - self_width) / 2)
        elseif align == "bottom" then
            x = other_props.x + (other_width - self_width)
        end
    else
        if dir == "left" then
            x = other_props.x - self_width
        else
            x = other_props.x + other_width
        end

        if align == "top" then
            y = other_props.y
        elseif align == "center" then
            y = other_props.y + math.floor((other_height - self_height) / 2)
        elseif align == "bottom" then
            y = other_props.y + (other_height - self_height)
        end
    end

    self:set_location({ x = x, y = y })
end

---Set this output's mode.
---
---If `refresh_rate_millihz` is provided, Pinnacle will attempt to use the mode with that refresh rate.
---If it isn't, Pinnacle will attempt to use the mode with the highest refresh rate that matches the
---given size.
---
---The refresh rate is in millihertz. For example, to choose a mode with a refresh rate of 60Hz, use 60000.
---
---If this output doesn't support the given mode, it will be ignored.
---
---### Example
---```lua
---Output.get_focused():set_mode(2560, 1440, 144000)
---```
---
---@param pixel_width integer
---@param pixel_height integer
---@param refresh_rate_millihz integer?
function OutputHandle:set_mode(pixel_width, pixel_height, refresh_rate_millihz)
    client.unary_request(build_grpc_request_params("SetMode", {
        output_name = self.name,
        pixel_width = pixel_width,
        pixel_height = pixel_height,
        refresh_rate_millihz = refresh_rate_millihz,
    }))
end

---Set this output's scaling factor.
---
---@param scale number
function OutputHandle:set_scale(scale)
    client.unary_request(build_grpc_request_params("SetScale", { output_name = self.name, absolute = scale }))
end

---Increase this output's scaling factor.
---
---@param increase_by number
function OutputHandle:increase_scale(increase_by)
    client.unary_request(build_grpc_request_params("SetScale", { output_name = self.name, relative = increase_by }))
end

---Decrease this output's scaling factor.
---
---@param decrease_by number
function OutputHandle:decrease_scale(decrease_by)
    client.unary_request(build_grpc_request_params("SetScale", { output_name = self.name, relative = -decrease_by }))
end

---@class Mode
---@field pixel_width integer
---@field pixel_height integer
---@field refresh_rate_millihz integer

---@class OutputProperties
---@field make string?
---@field model string?
---@field x integer?
---@field y integer?
---@field logical_width integer?
---@field logical_height integer?
---@field current_mode Mode?
---@field preferred_mode Mode?
---@field modes Mode[]
---@field physical_width integer?
---@field physical_height integer?
---@field focused boolean?
---@field tags TagHandle[]
---@field scale number?

---Get all properties of this output.
---
---@return OutputProperties
function OutputHandle:props()
    local response = client.unary_request(build_grpc_request_params("GetProperties", { output_name = self.name }))

    ---@diagnostic disable-next-line: invisible
    local handles = require("pinnacle.tag").handle.new_from_table(response.tag_ids or {})

    response.tags = handles
    response.tag_ids = nil
    response.modes = response.modes or {}

    return response
end

---Get this output's make.
---
---Note: make and model detection are currently somewhat iffy and may not work.
---
---Shorthand for `handle:props().make`.
---
---@return string?
function OutputHandle:make()
    return self:props().make
end

---Get this output's model.
---
---Note: make and model detection are currently somewhat iffy and may not work.
---
---Shorthand for `handle:props().model`.
---
---@return string?
function OutputHandle:model()
    return self:props().model
end

---Get this output's x-coordinate in the global space.
---
---Shorthand for `handle:props().x`.
---
---@return integer?
function OutputHandle:x()
    return self:props().x
end

---Get this output's y-coordinate in the global space.
---
---Shorthand for `handle:props().y`.
---
---@return integer?
function OutputHandle:y()
    return self:props().y
end

---Get this output's logical width in pixels.
---
---Shorthand for `handle:props().logical_width`.
---
---@return integer?
function OutputHandle:logical_width()
    return self:props().logical_width
end

---Get this output's logical height in pixels.
---
---Shorthand for `handle:props().y`.
---
---@return integer?
function OutputHandle:logical_height()
    return self:props().logical_height
end

---Get this output's current mode.
---
---Shorthand for `handle:props().current_mode`.
---
---@return Mode?
function OutputHandle:current_mode()
    return self:props().current_mode
end

---Get this output's preferred mode.
---
---Shorthand for `handle:props().preferred_mode`.
---
---@return Mode?
function OutputHandle:preferred_mode()
    return self:props().preferred_mode
end

---Get all of this output's available modes.
---
---Shorthand for `handle:props().modes`.
---
---@return Mode[]
function OutputHandle:modes()
    return self:props().modes
end

---Get this output's physical width in millimeters.
---
---Shorthand for `handle:props().physical_width`.
---
---@return integer?
function OutputHandle:physical_width()
    return self:props().physical_width
end

---Get this output's physical height in millimeters.
---
---Shorthand for `handle:props().physical_height`.
---
---@return integer?
function OutputHandle:physical_height()
    return self:props().physical_height
end

---Get whether or not this output is focused.
---
---The focused output is currently implemented as the one that last had pointer motion.
---
---Shorthand for `handle:props().focused`.
---
---@return boolean?
function OutputHandle:focused()
    return self:props().focused
end

---Get the tags this output has.
---
---Shorthand for `handle:props().tags`.
---
---@return TagHandle[]?
function OutputHandle:tags()
    return self:props().tags
end

---Get this output's scaling factor.
---
---Shorthand for `handle:props().scale`.
---
---@return number?
function OutputHandle:scale()
    return self:props().scale
end

---@nodoc
---Create a new `OutputHandle` from its raw name.
---@param output_name string
function output_handle.new(output_name)
    ---@type OutputHandle
    local self = {
        name = output_name,
    }
    setmetatable(self, { __index = OutputHandle })
    return self
end

return output
