-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local output_v1 = require("pinnacle.grpc.defs").pinnacle.output.v1

---@lcat nodoc
---@class pinnacle.output.OutputHandleModule
local output_handle = {}

---An output handle.
---
---This is a handle to one of your monitors.
---
---This can be retrieved through the various `get` functions in the `Output` module.
---
---@class pinnacle.output.OutputHandle
---@field name string The unique name of this output
local OutputHandle = {}

---Output management.
---
---An output is the Wayland term for a monitor. It presents windows, your cursor, and other UI elements.
---
---Outputs are uniquely identified by their name, a.k.a. the name of the connector they're plugged in to.
---
---@class pinnacle.output
---@lcat nodoc
---@field private handle pinnacle.output.OutputHandleModule
local output = {}
output.handle = output_handle

---Gets all outputs.
---
---@return pinnacle.output.OutputHandle[]
function output.get_all()
    local response, err = client:pinnacle_output_v1_OutputService_Get({})

    if err then
        log.error(err)
        return {}
    end

    ---@cast response pinnacle.output.v1.GetResponse

    ---@type pinnacle.output.OutputHandle[]
    local handles = {}

    for _, output_name in ipairs(response.output_names or {}) do
        table.insert(handles, output_handle.new(output_name))
    end

    return handles
end

---Gets all enabled outputs.
---
---@return pinnacle.output.OutputHandle[]
function output.get_all_enabled()
    local outputs = output.get_all()

    local enabled_handles = {}
    for _, handle in ipairs(outputs) do
        if handle:enabled() then
            table.insert(enabled_handles, handle)
        end
    end

    return enabled_handles
end

---Gets an output by its name (the connector it's plugged into).
---
---@param name string The output's name.
---
---@return pinnacle.output.OutputHandle | nil
function output.get_by_name(name)
    local handles = output.get_all()

    for _, handle in ipairs(handles) do
        if handle.name == name then
            return handle
        end
    end

    return nil
end

---Gets the currently focused output.
---
---This is currently defined as the most recent one that has had pointer motion.
---
---@return pinnacle.output.OutputHandle | nil
function output.get_focused()
    local handles = output.get_all()

    for _, handle in ipairs(handles) do
        if handle:focused() then
            return handle
        end
    end

    return nil
end

--- Runs a function on all current and future outputs.
---
--- When called, this will do two things:
--- 1. Immediately run `for_each` with all currently connected outputs.
--- 2. Call `for_each` with any newly connected outputs.
---
--- Note that `for_each` will *not* run with outputs that have been unplugged and replugged.
--- This is to prevent duplicate setup. Instead, the compositor keeps track of any tags and
--- state the output had when unplugged and restores them on replug. This may change in the future.
---
---#### Example
---```lua
--- -- Add tags "1" through "5" to all outputs
---require("pinnacle.output").for_each_output(function(output)
---    local tags = Tag.add(output, "1", "2", "3", "4", "5")
---    tags[1]:toggle_active()
---end)
---```
---
---@param for_each fun(output: pinnacle.output.OutputHandle) The function that will be run for each output.
function output.for_each_output(for_each)
    local handles = output.get_all()
    for _, handle in ipairs(handles) do
        for_each(handle)
    end

    output.connect_signal({
        connect = for_each,
    })
end

local signal_name_to_SignalName = {
    connect = "OutputConnect",
    disconnect = "OutputDisconnect",
    resize = "OutputResize",
    move = "OutputMove",
}

---@class pinnacle.output.OutputSignal Signals related to output events.
---@field connect fun(output: pinnacle.output.OutputHandle)? An output was connected. FIXME: This currently does not fire for outputs that have been previously connected and disconnected.
---@field disconnect fun(output: pinnacle.output.OutputHandle)? An output was disconnected.
---@field resize fun(output: pinnacle.output.OutputHandle, logical_width: integer, logical_height: integer)? An output's logical size changed.
---@field move fun(output: pinnacle.output.OutputHandle, x: integer, y: integer)? An output moved.

---Connects to an output signal.
---
---`signals` is a table containing the signal(s) you want to connect to along with
---a corresponding callback that will be called when the signal is signalled.
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
---@param signals pinnacle.output.OutputSignal The signal you want to connect to
---
---@return pinnacle.signal.SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see pinnacle.signal.SignalHandles.disconnect_all - To disconnect from these signals
function output.connect_signal(signals)
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

---------------------------------------------------------------------

---Sets the location of this output in the global space.
---
---On startup, Pinnacle will lay out all connected outputs starting at (0, 0)
---and going to the right, with their top borders aligned.
---
---This method allows you to move outputs where necessary.
---
---Note: If you have space between two outputs when setting their locations,
---the pointer will not be able to move between them.
---
---#### Example
---```lua
--- -- Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
--- --  - "DP-1":   ┌─────┐
--- --              │     │1920x1080
--- --              └─────┘
--- --  - "HDMI-1": ┌───────┐
--- --              │ 2560x │
--- --              │ 1440  │
--- --              └───────┘
---Output.get_by_name("DP-1"):set_loc(0, 0)
---Output.get_by_name("HDMI-1"):set_loc(1920, -360)
--- -- Results in:
--- --       ┌───────┐
--- -- ┌─────┤       │
--- -- │DP-1 │HDMI-1 │
--- -- └─────┴───────┘
--- -- Notice that y = 0 aligns with the top of "DP-1", and the top of "HDMI-1" is at y = -360.
---```
---
---@param x integer The x-coordinate.
---@param y integer The y-coordinate.
---
---@see pinnacle.output.OutputHandle.set_loc_adj_to A helper function to move outputs relative to other outputs.
function OutputHandle:set_loc(x, y)
    local _, err = client:pinnacle_output_v1_OutputService_SetLoc({
        output_name = self.name,
        x = x,
        y = y,
    })

    if err then
        log.error(err)
    end
end

---An alignment relative to another output.
---@alias pinnacle.output.Alignment
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

---Sets the location of this output adjacent to another one.
---
---`alignment` is how you want this output to be placed.
---For example, "top_align_left" will place this output above `other` and align the left borders.
---Similarly, "right_align_center" will place this output to the right of `other` and align their centers.
---
---#### Example
---```lua
--- -- Assume two monitors in order, "DP-1" and "HDMI-1", with the following dimensions:
--- --  - "DP-1":   ┌─────┐
--- --              │     │1920x1080
--- --              └─────┘
--- --  - "HDMI-1": ┌───────┐
--- --              │ 2560x │
--- --              │ 1440  │
--- --              └───────┘
---Output.get_by_name("DP-1"):set_loc_adj_to(Output.get_by_name("HDMI-1"), "bottom_align_right")
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
---@param other pinnacle.output.OutputHandle The output to move this output relative to.
---@param alignment pinnacle.output.Alignment How to align this output with the other output.
function OutputHandle:set_loc_adj_to(other, alignment)
    local self_logical_size = self:logical_size()
    local other_logical_size = other:logical_size()
    local other_loc = other:loc()

    if not self_logical_size or not other_logical_size or not other_loc then
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

    local self_width = self_logical_size.width
    local self_height = self_logical_size.height
    local other_width = other_logical_size.width
    local other_height = other_logical_size.height

    if not (self_width and self_height and other_width and other_height) then
        return
    end

    if dir == "top" or dir == "bottom" then
        if dir == "top" then
            y = other_loc.y - self_height
        else
            y = other_loc.y + other_height
        end

        if align == "left" then
            x = other_loc.x
        elseif align == "center" then
            x = other_loc.x + math.floor((other_width - self_width) / 2)
        elseif align == "bottom" then
            x = other_loc.x + (other_width - self_width)
        end
    else
        if dir == "left" then
            x = other_loc.x - self_width
        else
            x = other_loc.x + other_width
        end

        if align == "top" then
            y = other_loc.y
        elseif align == "center" then
            y = other_loc.y + math.floor((other_height - self_height) / 2)
        elseif align == "bottom" then
            y = other_loc.y + (other_height - self_height)
        end
    end

    self:set_loc(x, y)
end

---Sets this output's mode.
---
---If `refresh_rate_mhz` is provided, Pinnacle will attempt to use the mode with that refresh rate.
---If it isn't, Pinnacle will attempt to use the mode with the highest refresh rate that matches the
---given size.
---
---The refresh rate is in millihertz. For example, to choose a mode with a refresh rate of 60Hz, use 60000.
---
---If this output doesn't support the given mode, it will be ignored.
---
---#### Example
---```lua
---Output.get_focused():set_mode(2560, 1440, 144000)
---```
---
---@param width integer The mode's width.
---@param height integer The mode's height.
---@param refresh_rate_mhz integer? The mode's refresh rate in millihertz, or `nil` to auto-select.
function OutputHandle:set_mode(width, height, refresh_rate_mhz)
    local _, err = client:pinnacle_output_v1_OutputService_SetMode({
        output_name = self.name,
        size = { width = width, height = height },
        refresh_rate_mhz = refresh_rate_mhz,
        custom = false,
    })

    if err then
        log.error(err)
    end
end

---Sets this output's mode to a custom one.
---
---If `refresh_rate_mhz` is provided, Pinnacle will create a new mode with that refresh rate.
---If it isn't, it will default to 60Hz.
---
---The refresh rate is in millihertz. For example, to choose a mode with a refresh rate of 60Hz, use 60000.
---
---#### Example
---```lua
---Output.get_focused():set_custom_mode(2560, 1440, 75000)
---```
---
---@param width integer A custom width.
---@param height integer A custom height.
---@param refresh_rate_mhz integer? A custom refresh rate in millihertz, or `nil` to default to 60Hz.
function OutputHandle:set_custom_mode(width, height, refresh_rate_mhz)
    local _, err = client:pinnacle_output_v1_OutputService_SetMode({
        output_name = self.name,
        size = { width = width, height = height },
        refresh_rate_mhz = refresh_rate_mhz,
        custom = true,
    })

    if err then
        log.error(err)
    end
end

---A custom modeline.
---@class pinnacle.output.Modeline
---@field clock number
---@field hdisplay integer
---@field hsync_start integer
---@field hsync_end integer
---@field htotal integer
---@field vdisplay integer
---@field vsync_start integer
---@field vsync_end integer
---@field vtotal integer
---@field hsync boolean
---@field vsync boolean

---Sets a custom modeline for this output.
---
---This accepts a `Modeline` table or a string of the modeline.
---You can parse a modeline into a `Modeline` table with
---```lua
---require("pinnacle.util").output.parse_modeline(
---    "173.00 1920 2048 2248 2576 1080 1083 1088 1120 -hsync +vsync"
---)
---```
---
---@param modeline string|pinnacle.output.Modeline A modeline table, or a modeline string to feed it into `parse_modeline`.
---
---@see pinnacle.util.output.parse_modeline
function OutputHandle:set_modeline(modeline)
    if type(modeline) == "string" then
        local ml, err = require("pinnacle.util").output.parse_modeline(modeline)
        if ml then
            modeline = ml
        else
            print("invalid modeline: " .. tostring(err))
            return
        end
    end

    ---@type pinnacle.output.v1.SetModelineRequest
    local request = {
        output_name = self.name,
        modeline = {
            clock = modeline.clock,
            hdisplay = modeline.hdisplay,
            hsync_start = modeline.hsync_start,
            hsync_end = modeline.hsync_end,
            htotal = modeline.htotal,
            vdisplay = modeline.vdisplay,
            vsync_start = modeline.vsync_start,
            vsync_end = modeline.vsync_end,
            vtotal = modeline.vtotal,
            hsync = modeline.hsync,
            vsync = modeline.vsync,
        },
    }

    local _, err = client:pinnacle_output_v1_OutputService_SetModeline(request)

    if err then
        log.error(err)
    end
end

---Sets this output's scaling factor.
---
---@param scale number The new scale.
function OutputHandle:set_scale(scale)
    local _, err = client:pinnacle_output_v1_OutputService_SetScale({
        output_name = self.name,
        scale = scale,
        abs_or_rel = require("pinnacle.grpc.defs").pinnacle.util.v1.AbsOrRel.ABS_OR_REL_ABSOLUTE,
    })

    if err then
        log.error(err)
    end
end

---Changes this output's scaling factor by the given amount.
---
---@param change_by number How much to change the current scale by.
function OutputHandle:change_scale(change_by)
    local _, err = client:pinnacle_output_v1_OutputService_SetScale({
        output_name = self.name,
        scale = change_by,
        abs_or_rel = require("pinnacle.grpc.defs").pinnacle.util.v1.AbsOrRel.ABS_OR_REL_RELATIVE,
    })

    if err then
        log.error(err)
    end
end

---An output transform.
---
---This determines what orientation outputs will render with.
---@enum (key) pinnacle.output.Transform
local transform_name_to_code = {
    ---No transform.
    normal = output_v1.Transform.TRANSFORM_NORMAL,
    ---90 degrees counter-clockwise.
    ["90"] = output_v1.Transform.TRANSFORM_90,
    ---180 degrees counter-clockwise.
    ["180"] = output_v1.Transform.TRANSFORM_180,
    ---270 degrees counter-clockwise.
    ["270"] = output_v1.Transform.TRANSFORM_270,
    ---Flipped vertically (across the horizontal axis).
    flipped = output_v1.Transform.TRANSFORM_FLIPPED,
    ---Flipped vertically and rotated 90 degrees counter-clockwise.
    flipped_90 = output_v1.Transform.TRANSFORM_FLIPPED_90,
    ---Flipped vertically and rotated 180 degrees counter-clockwise.
    flipped_180 = output_v1.Transform.TRANSFORM_FLIPPED_180,
    ---Flipped vertically and rotated 270 degrees counter-clockwise.
    flipped_270 = output_v1.Transform.TRANSFORM_FLIPPED_270,
}
require("pinnacle.util").make_bijective(transform_name_to_code)

---Sets this output's transform.
---
---@param transform pinnacle.output.Transform The new transform.
function OutputHandle:set_transform(transform)
    local _, err = client:pinnacle_output_v1_OutputService_SetTransform({
        output_name = self.name,
        transform = transform_name_to_code[transform],
    })

    if err then
        log.error(err)
    end
end

local set_or_toggle = {
    SET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    [true] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    UNSET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    [false] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    TOGGLE = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_TOGGLE,
}

---Powers on or off this output.
---
---@param powered boolean
function OutputHandle:set_powered(powered)
    local _, err = client:pinnacle_output_v1_OutputService_SetPowered({
        output_name = self.name,
        set_or_toggle = set_or_toggle[powered],
    })

    if err then
        log.error(err)
    end
end

---Toggles power for this output.
function OutputHandle:toggle_powered()
    local _, err = client:pinnacle_output_v1_OutputService_SetPowered({
        output_name = self.name,
        set_or_toggle = set_or_toggle.TOGGLE,
    })

    if err then
        log.error(err)
    end
end

---An output pixel dimension and refresh rate configuration.
---@class pinnacle.output.Mode
---The width of the mode, in pixels.
---@field width integer
---The height of the mode, in pixels.
---@field height integer
---The output's refresh rate, in millihertz.
---@field refresh_rate_mhz integer

---Gets this output's make.
---
---@return string # The make, or an empty string if it doesn't have one.
function OutputHandle:make()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetInfo({ output_name = self.name })

    return response and response.make or ""
end

---Gets this output's model.
---
---@return string # The model, or an empty string if it doesn't have one.
function OutputHandle:model()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetInfo({ output_name = self.name })

    return response and response.model or ""
end

---Gets this output's serial.
---
---@return string # The serial, or an empty string if it doesn't have one.
function OutputHandle:serial()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetInfo({ output_name = self.name })

    return response and response.serial or ""
end

---Gets this output's location in the global space.
---
---@return { x: integer, y: integer }? # The output's location, or `nil` if it is not enabled or doesn't exist.
function OutputHandle:loc()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetLoc({ output_name = self.name })

    return response and response.loc
end

---Gets this output's logical size in logical pixels.
---
---@return { width: integer, height: integer }? # The output's logical size, or `nil` if it is disabled or doesn't exist.
function OutputHandle:logical_size()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetLogicalSize({ output_name = self.name })

    return response and response.logical_size
end

---Gets this output's physical size in millimeters.
---
---@return { width: integer, height: integer }? # The output's physical size, or `nil` if it doesn't advertise one or doesn't exist.
function OutputHandle:physical_size()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetPhysicalSize({ output_name = self.name })

    return response and response.physical_size
end

---Gets this output's current mode.
---
---@return pinnacle.output.Mode? # The current mode, or `nil` if the output is disabled or doesn't exist.
function OutputHandle:current_mode()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetModes({ output_name = self.name })

    local mode = response and response.current_mode
    if not mode then
        return nil
    end

    ---@type pinnacle.output.Mode
    local ret = {
        width = mode.size.width,
        height = mode.size.height,
        refresh_rate_mhz = mode.refresh_rate_mhz,
    }

    return ret
end

---Gets this output's preferred mode.
---
---@return pinnacle.output.Mode? # The preferred mode, or `nil` if the output doesn't exist.
function OutputHandle:preferred_mode()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetModes({ output_name = self.name })

    local mode = response and response.preferred_mode
    if not mode then
        return nil
    end

    ---@type pinnacle.output.Mode
    local ret = {
        width = mode.size.width,
        height = mode.size.height,
        refresh_rate_mhz = mode.refresh_rate_mhz,
    }

    return ret
end

---Gets all of this output's available modes.
---
---@return pinnacle.output.Mode[]
function OutputHandle:modes()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetModes({ output_name = self.name })

    local modes = response and response.modes
    if not modes then
        return {}
    end

    ---@type pinnacle.output.Mode[]
    local ret = {}

    for _, mode in ipairs(modes) do
        ---@type pinnacle.output.Mode
        local md = {
            width = mode.size.width,
            height = mode.size.height,
            refresh_rate_mhz = mode.refresh_rate_mhz,
        }
        table.insert(ret, md)
    end

    return ret
end

---Gets whether or not this output is focused.
---
---The focused output is currently implemented as the one that last had pointer motion.
---
---@return boolean
function OutputHandle:focused()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetFocused({ output_name = self.name })

    return response and response.focused or false
end

---Gets the tags this output has.
---
---@return pinnacle.tag.TagHandle[]
function OutputHandle:tags()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetTagIds({ output_name = self.name })

    local tag_ids = response and response.tag_ids or {}

    local handles = require("pinnacle.tag").handle.new_from_table(tag_ids)

    return handles
end

---Get this output's scale.
---
---@return number
function OutputHandle:scale()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetScale({ output_name = self.name })

    return response and response.scale or 1.0
end

---Get this output's transform.
---
---@return pinnacle.output.Transform
function OutputHandle:transform()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetTransform({ output_name = self.name })

    local transform = (
        response and response.transform
        or require("pinnacle.grpc.defs").pinnacle.output.v1.Transform.TRANSFORM_NORMAL
    )

    ---@type pinnacle.output.Transform
    return transform_name_to_code[transform]
end

---Gets whether this output is enabled.
---
---Disabled outputs are not mapped to the global space and cannot be used.
---
---@return boolean
function OutputHandle:enabled()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetEnabled({ output_name = self.name })

    return response and response.enabled or false
end

---Gets whether this output is powered.
---
---Unpowered outputs that are enabled will be off, but they will still be
---mapped to the global space, meaning you can still interact with them.
---
---@return boolean
function OutputHandle:powered()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetPowered({ output_name = self.name })

    return response and response.powered or false
end

---Gets this output's keyboard focus stack.
---
---This includes *all* windows on the output, even those on inactive tags.
---If you only want visible windows, use `keyboard_focus_stack_visible` instead.
---
---@return pinnacle.window.WindowHandle[]
---
---@see pinnacle.output.OutputHandle.keyboard_focus_stack_visible
function OutputHandle:keyboard_focus_stack()
    local response, err =
        client:pinnacle_output_v1_OutputService_GetFocusStackWindowIds({ output_name = self.name })

    local window_ids = response and response.window_ids or {}

    local handles = require("pinnacle.window").handle.new_from_table(window_ids)

    return handles
end

---Gets this output's keyboard focus stack.
---
---This only includes windows on active tags.
---If you want all windows on the output, use `keyboard_focus_stack` instead.
---
---@return pinnacle.window.WindowHandle[]
---
---@see pinnacle.output.OutputHandle.keyboard_focus_stack
function OutputHandle:keyboard_focus_stack_visible()
    local stack = self:keyboard_focus_stack()

    ---@type (fun(): boolean)[]
    local batch = {}
    for i, win in ipairs(stack) do
        batch[i] = function()
            return win:is_on_active_tag()
        end
    end

    local on_active_tags = require("pinnacle.util").batch(batch)

    ---@type pinnacle.window.WindowHandle[]
    local keyboard_focus_stack_visible = {}

    for i, is_active in ipairs(on_active_tags) do
        if is_active then
            table.insert(keyboard_focus_stack_visible, stack[i])
        end
    end

    return keyboard_focus_stack_visible
end

---Creates a new `OutputHandle` from its raw name.
---@param output_name string
function output_handle.new(output_name)
    ---@type pinnacle.output.OutputHandle
    local self = {
        name = output_name,
    }
    setmetatable(self, { __index = OutputHandle })
    return self
end

return output
