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

local signal_name_to_SignalName = {
    connect = "OutputConnect",
}

---@class OutputSignal Signals related to output events.
---@field connect fun(output: OutputHandle)? An output was connected. FIXME: This currently does not fire for outputs that have been previously connected and disconnected.

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

    if not self_props.current_mode or not other_props.current_mode then
        return
    end

    local self_width = self_props.current_mode.pixel_width
    local self_height = self_props.current_mode.pixel_height
    local other_width = other_props.current_mode.pixel_width
    local other_height = other_props.current_mode.pixel_height

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
---@field current_mode Mode?
---@field preferred_mode Mode?
---@field modes Mode[]
---@field physical_width integer?
---@field physical_height integer?
---@field focused boolean?
---@field tags TagHandle[]

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
