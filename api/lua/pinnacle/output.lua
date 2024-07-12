-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local output_v0alpha1 = require("pinnacle.grpc.defs").pinnacle.output.v0alpha1
local output_service = require("pinnacle.grpc.defs").pinnacle.output.v0alpha1.OutputService

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
    local response = client:unary_request(output_service.Get, {})

    ---@type OutputHandle[]
    local handles = {}

    for _, output_name in ipairs(response.output_names or {}) do
        table.insert(handles, output_handle.new(output_name))
    end

    return handles
end

---Get all enabled outputs.
---
---### Example
---```lua
---local outputs = Output.get_all_enabled()
---```
---
---@return OutputHandle[]
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

---@param id_str string
---@param op OutputHandle
---
---@return boolean
local function output_id_matches(id_str, op)
    if id_str:match("^serial:") then
        local serial = tonumber(id_str:sub(8))
        return serial and serial == op:serial() or false
    else
        return id_str == op.name
    end
end

---@class OutputSetup
---@field filter (fun(output: OutputHandle): boolean)? -- A filter for wildcard matches that should return true if this setup should apply to the passed in output.
---@field mode Mode? -- Makes this setup apply the given mode to outputs.
---@field modeline (string|Modeline)? -- Makes this setup apply the given modeline to outputs. This takes precedence over `mode`.
---@field scale number? -- Makes this setup apply the given scale to outputs.
---@field tags string[]? -- Makes this setup add tags with the given name to outputs.
---@field transform Transform? -- Makes this setup applt the given transform to outputs.

---Declaratively setup outputs.
---
---`Output.setup` allows you to specify output properties that will be applied immediately and
---on output connection. These include mode, scale, tags, and more.
---
---`setups` is a table of output identifier strings to `OutputSetup`s.
---
---### Keys
---
---Keys attempt to match outputs.
---
---Wildcard keys (`"*"`) will match all outputs. You can additionally filter these outputs
---by setting a `filter` function in the setup that returns true if it should apply to the output.
---(See the example.)
---
---Otherwise, keys will attempt to match the exact name of an output.
---
---Use "serial:<number>" to match outputs by their EDID serial. For example, "serial:143256".
---Note that not all displays have EDID serials. Also, serials are not guaranteed to be unique.
---If you're unlucky enough to have two displays with the same serial, you'll have to use their names
---or filter with wildcards instead.
---
---### Setups
---
---If an output is matched, the corresponding `OutputSetup` entry will be applied to it.
---Any given `tags` will be added, and things like `transform`s, `scale`s, and `mode`s will be set.
---
---### Ordering setups
---
---You may need to specify multiple wildcard matches for different setup applications.
---You can't just add another key of `"*"`, because that would overwrite the old `"*"`.
---In this case, you can order setups by prepending `n:` to the key, where n is an ordering number.
---`n` should be between `1` and `#setups`. Setting higher orders without setting lower ones
---will cause entries without orders to fill up lower numbers in an arbitrary order. Setting
---orders above `#setups` may cause their entries to not apply.
---
---
---### Example
---```lua
---Output.setup({
---    -- Give all outputs tags 1 through 5
---    ["1:*"] = {
---        tags = { "1", "2", "3", "4", "5" },
---    },
---    -- Give outputs with a preferred mode of 4K a scale of 2.0
---    ["2:*"] = {
---        filter = function(op)
---            return op:preferred_mode().pixel_height == 2160
---        end,
---        scale = 2.0,
---    },
---    -- Additionally give eDP-1 tags 6 and 7
---    ["eDP-1"] = {
---        tags = { "6", "7" },
---    },
---    -- Match an output by its EDID serial number
---    ["serial:235987"] = { ... }
---})
---```
---
---@param setups table<string, OutputSetup>
function output.setup(setups)
    ---@type { [1]: string, setup: OutputSetup }[]
    local op_setups = {}

    local setup_len = 0

    -- Index entries with an index
    for op_id, op_setup in pairs(setups) do
        setup_len = setup_len + 1

        ---@type string|nil
        if op_id:match("^%d+:") then
            ---@type string
            local index = op_id:match("^%d+")
            ---@diagnostic disable-next-line: redefined-local
            local op_id = op_id:sub(index:len() + 2)
            ---@diagnostic disable-next-line: redefined-local
            local index = tonumber(index)

            ---@cast index number

            op_setups[index] = { op_id, setup = op_setup }
        end
    end

    -- Insert *s first
    for op_id, op_setup in pairs(setups) do
        if op_id:match("^*$") then
            -- Fill up holes if there are any
            for i = 1, setup_len do
                if not op_setups[i] then
                    op_setups[i] = { op_id, setup = op_setup }
                    break
                end
            end
        end
    end

    -- Insert rest of the entries
    for op_id, op_setup in pairs(setups) do
        if not op_id:match("^%d+:") and op_id ~= "*" then
            -- Fill up holes if there are any
            for i = 1, setup_len do
                if not op_setups[i] then
                    op_setups[i] = { op_id, setup = op_setup }
                    break
                end
            end
        end
    end

    ---@param op OutputHandle
    local function apply_setups(op)
        for _, op_setup in ipairs(op_setups) do
            if output_id_matches(op_setup[1], op) or op_setup[1] == "*" then
                local setup = op_setup.setup

                if setup.filter and not setup.filter(op) then
                    goto continue
                end

                if setup.modeline then
                    op:set_modeline(setup.modeline)
                elseif setup.mode then
                    op:set_mode(
                        setup.mode.pixel_width,
                        setup.mode.pixel_height,
                        setup.mode.refresh_rate_millihz
                    )
                end
                if setup.scale then
                    op:set_scale(setup.scale)
                end
                if setup.tags then
                    require("pinnacle.tag").add(op, setup.tags)
                end
                if setup.transform then
                    op:set_transform(setup.transform)
                end
            end

            ::continue::
        end

        local tags = op:tags() or {}
        if tags[1] then
            tags[1]:set_active(true)
        end
    end

    output.connect_for_all(function(op)
        apply_setups(op)
    end)
end

---@alias OutputLoc
---| { [1]: integer, [2]: integer } -- A specific point
---| { [1]: string, [2]: Alignment } -- A location relative to another output

---@alias UpdateLocsOn
---| "connect" -- Update output locations on output connect
---| "disconnect" -- Update output locations on output disconnect
---| "resize" -- Update output locations on output resize

---Setup locations for outputs.
---
---This function lets you declare positions for outputs, either as a specific point in the global
---space or relative to another output.
---
---### Choosing when to recompute output positions
---
---`update_locs_on` specifies when output positions should be recomputed. It can be `"all"`, signaling you
---want positions to update on all of output connect, disconnect, and resize, or it can be a table
---containing `"connect"`, `"disconnect"`, and/or `"resize"`.
---
---### Specifying locations
---
---`locs` should be a table of output identifiers to locations.
---
---#### Output identifiers
---
---Keys for `locs` should be output identifiers. These are strings of
---the name of the output, for example "eDP-1" or "HDMI-A-1".
---Additionally, if you want to match the EDID serial of an output,
---prepend the serial with "serial:", for example "serial:174652".
---You can find this by doing `get-edid | edid-decode`.
---
---#### Fallback relative-tos
---
---Sometimes you have an output with a relative location, but the output
---it's relative to isn't connected. In this case you can specify an
---order that locations will be placed by prepending "n:" to the key.
---For example, "4:HDMI-1" will be applied before "5:HDMI-1", allowing
---you to specify more than one relative output. The first connected
---relative output will be chosen for placement. See the example below.
---
---### Example
---```lua
---               -- vvvvv Relayout on output connect, disconnect, and resize
---Output.setup_locs("all", {
---    -- Anchor eDP-1 to (0, 0) so we can place other outputs relative to it
---    ["eDP-1"] = { 0, 0 },
---    -- Place HDMI-A-1 below it centered
---    ["HDMI-A-1"] = { "eDP-1", "bottom_align_center" },
---    -- Place HDMI-A-2 below HDMI-A-1.
---    ["3:HDMI-A-2"] = { "HDMI-A-1", "bottom_align_center" },
---    -- Additionally, if HDMI-A-1 isn't connected, fallback to placing it below eDP-1 instead.
---    ["4:HDMI-A-2"] = { "eDP-1", "bottom_align_center" },
---
---    -- Note that the last two have a number followed by a colon. This dictates the order of application.
---    -- Because Lua tables with string keys don't index by declaration order, this is needed to specify that.
---    -- You can also put a "1:" and "2:" in front of "eDP-1" and "HDMI-A-1" if you want to be explicit
---    -- about their ordering.
---    --
---    -- Just note that orders must be from 1 to the length of the array. Entries without an order
---    -- will be filled in from 1 upwards, taking any open slots. Entries with orders above
---    -- #locs may not be applied.
---})
---
--- -- Only relayout on output connect and resize
---Output.setup_locs({ "connect", "resize" }, { ... })
---
--- -- Use EDID serials for identification.
--- -- You can run
--- -- require("pinnacle").run(function(Pinnacle)
--- --     print(Pinnacle.output.get_focused():serial())
--- -- end)
--- -- in a Lua repl to find the EDID serial of the focused output.
---Output.setup_locs("all" {
---    ["serial:139487"] = { ... },
---})
---```
---
---@param update_locs_on (UpdateLocsOn)[] | "all"
---@param locs table<string, OutputLoc>
function output.setup_locs(update_locs_on, locs)
    ---@type { [1]: string, loc: OutputLoc }[]
    local setups = {}

    local setup_len = 0

    -- Index entries with an index
    for op_id, op_loc in pairs(locs) do
        setup_len = setup_len + 1

        ---@type string|nil
        if op_id:match("^%d+:") then
            ---@type string
            local index = op_id:match("^%d+")
            ---@diagnostic disable-next-line: redefined-local
            local op_id = op_id:sub(index:len() + 2)
            ---@diagnostic disable-next-line: redefined-local
            local index = tonumber(index)

            ---@cast index number

            setups[index] = { op_id, loc = op_loc }
        end
    end

    -- Insert rest of the entries
    for op_id, op_loc in pairs(locs) do
        if not op_id:match("^%d+:") then
            -- Fill up holes if there are any
            for i = 1, setup_len do
                if not setups[i] then
                    setups[i] = { op_id, loc = op_loc }
                    break
                end
            end
        end
    end

    local function layout_outputs()
        local outputs = output.get_all_enabled()

        ---@type OutputHandle[]
        local placed_outputs = {}

        local rightmost_output = {
            output = nil,
            x = nil,
        }

        -- Place outputs with a specified location first
        ---@diagnostic disable-next-line: redefined-local
        for _, setup in ipairs(setups) do
            for _, op in ipairs(outputs) do
                if output_id_matches(setup[1], op) then
                    if type(setup.loc[1]) == "number" then
                        local loc = { x = setup.loc[1], y = setup.loc[2] }
                        op:set_location(loc)
                        table.insert(placed_outputs, op)

                        local props = op:props()
                        if
                            not rightmost_output.x
                            or rightmost_output.x < props.x + props.logical_width
                        then
                            rightmost_output.output = op
                            rightmost_output.x = props.x + props.logical_width
                        end
                    end
                    break
                end
            end
        end

        -- Place outputs that are relative to other outputs
        local function next_output_with_relative_to()
            ---@diagnostic disable-next-line: redefined-local
            for _, setup in ipairs(setups) do
                for _, op in ipairs(outputs) do
                    for _, placed_op in ipairs(placed_outputs) do
                        if placed_op.name == op.name then
                            goto continue
                        end
                    end

                    if not output_id_matches(setup[1], op) or type(setup.loc[1]) == "number" then
                        goto continue
                    end

                    local relative_to_name = setup.loc[1]
                    local alignment = setup.loc[2]
                    for _, placed_op in ipairs(placed_outputs) do
                        if placed_op.name == relative_to_name then
                            return op, placed_op, alignment
                        end
                    end

                    goto continue_outer

                    ::continue::
                end
                ::continue_outer::
            end

            return nil, nil, nil
        end

        while true do
            local op, relative_to, alignment = next_output_with_relative_to()
            if not op then
                break
            end

            ---@cast relative_to OutputHandle
            ---@cast alignment Alignment

            op:set_loc_adj_to(relative_to, alignment)
            table.insert(placed_outputs, op)

            local props = op:props()
            if not rightmost_output.x or rightmost_output.x < props.x + props.logical_width then
                rightmost_output.output = op
                rightmost_output.x = props.x + props.logical_width
            end
        end

        -- Place still-not-placed outputs
        for _, op in ipairs(outputs) do
            for _, placed_op in ipairs(placed_outputs) do
                if placed_op.name == op.name then
                    goto continue
                end
            end

            if not rightmost_output.output then
                op:set_location({ x = 0, y = 0 })
            else
                op:set_loc_adj_to(rightmost_output.output, "right_align_top")
            end

            local props = op:props()

            rightmost_output.output = op
            rightmost_output.x = props.x

            table.insert(placed_outputs, op)

            ::continue::
        end
    end

    layout_outputs()

    local layout_on_connect = false
    local layout_on_disconnect = false
    local layout_on_resize = false

    if update_locs_on == "all" then
        layout_on_connect = true
        layout_on_disconnect = true
        layout_on_resize = true
    else
        ---@cast update_locs_on UpdateLocsOn[]

        for _, update_on in ipairs(update_locs_on) do
            if update_on == "connect" then
                layout_on_connect = true
            elseif update_on == "disconnect" then
                layout_on_disconnect = true
            elseif update_on == "resize" then
                layout_on_resize = true
            end
        end
    end

    if layout_on_connect then
        -- FIXME: This currently does not duplicate tags because the connect signal does not fire for
        -- |      previously connected outputs. However, this is unintended behavior, so fix this when you fix that.
        output.connect_signal({
            connect = function(_)
                layout_outputs()
            end,
        })
    end
    if layout_on_disconnect then
        output.connect_signal({
            disconnect = function(_)
                layout_outputs()
            end,
        })
    end
    if layout_on_resize then
        output.connect_signal({
            resize = function(_)
                layout_outputs()
            end,
        })
    end
end

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
        local handle =
            ---@diagnostic disable-next-line: invisible
            require("pinnacle.signal").handle.new(signal_name_to_SignalName[signal], callback)
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
    client:unary_request(output_service.SetLocation, {
        output_name = self.name,
        x = loc.x,
        y = loc.y,
    })
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
    client:unary_request(output_service.SetMode, {
        output_name = self.name,
        pixel_width = pixel_width,
        pixel_height = pixel_height,
        refresh_rate_millihz = refresh_rate_millihz,
    })
end

---@class Modeline
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

---Set a custom modeline for this output.
---
---This accepts a `Modeline` table or a string of the modeline.
---
---@param modeline string|Modeline
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

    ---@type pinnacle.output.v0alpha1.SetModelineRequest
    local request = {
        output_name = self.name,
        clock = modeline.clock,
        hdisplay = modeline.hdisplay,
        hsync_start = modeline.hsync_start,
        hsync_end = modeline.hsync_end,
        htotal = modeline.htotal,
        vdisplay = modeline.vdisplay,
        vsync_start = modeline.vsync_start,
        vsync_end = modeline.vsync_end,
        vtotal = modeline.vtotal,
        hsync_pos = modeline.hsync,
        vsync_pos = modeline.vsync,
    }

    client:unary_request(output_service.SetModeline, request)
end

---Set this output's scaling factor.
---
---@param scale number
function OutputHandle:set_scale(scale)
    client:unary_request(output_service.SetScale, { output_name = self.name, absolute = scale })
end

---Increase this output's scaling factor.
---
---@param increase_by number
function OutputHandle:increase_scale(increase_by)
    client:unary_request(
        output_service.SetScale,
        { output_name = self.name, relative = increase_by }
    )
end

---Decrease this output's scaling factor.
---
---@param decrease_by number
function OutputHandle:decrease_scale(decrease_by)
    self:increase_scale(-decrease_by)
end

---@enum (key) Transform
local transform_name_to_code = {
    normal = output_v0alpha1.Transform.TRANSFORM_NORMAL,
    ["90"] = output_v0alpha1.Transform.TRANSFORM_90,
    ["180"] = output_v0alpha1.Transform.TRANSFORM_180,
    ["270"] = output_v0alpha1.Transform.TRANSFORM_270,
    flipped = output_v0alpha1.Transform.TRANSFORM_FLIPPED,
    flipped_90 = output_v0alpha1.Transform.TRANSFORM_FLIPPED_90,
    flipped_180 = output_v0alpha1.Transform.TRANSFORM_FLIPPED_180,
    flipped_270 = output_v0alpha1.Transform.TRANSFORM_FLIPPED_270,
}
require("pinnacle.util").make_bijective(transform_name_to_code)

---Set this output's transform.
---
---@param transform Transform
function OutputHandle:set_transform(transform)
    client:unary_request(
        output_service.SetTransform,
        { output_name = self.name, transform = transform_name_to_code[transform] }
    )
end

---Power on or off this output.
---
---@param powered boolean
function OutputHandle:set_powered(powered)
    client:unary_request(output_service.SetPowered, { output_name = self.name, powered = powered })
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
---@field transform Transform?
---@field serial integer?
---@field keyboard_focus_stack WindowHandle[]
---@field enabled boolean?
---@field powered boolean?

---Get all properties of this output.
---
---@return OutputProperties
function OutputHandle:props()
    ---@type pinnacle.output.v0alpha1.GetPropertiesResponse
    local response = client:unary_request(output_service.GetProperties, { output_name = self.name })

    ---@diagnostic disable-next-line: invisible
    local tag_handles = require("pinnacle.tag").handle.new_from_table(response.tag_ids or {})

    ---@diagnostic disable-next-line: invisible
    local keyboard_focus_stack_handles = require("pinnacle.window").handle.new_from_table(
        response.keyboard_focus_stack_window_ids or {}
    )

    response.tag_ids = nil

    -- hehe
    ---@diagnostic disable-next-line: cast-type-mismatch
    ---@cast response OutputProperties

    response.tags = tag_handles
    response.modes = response.modes or {}
    response.transform = transform_name_to_code[response.transform]
    response.keyboard_focus_stack = keyboard_focus_stack_handles

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
---If the output is disabled, this returns nil.
---
---Shorthand for `handle:props().logical_width`.
---
---@return integer?
function OutputHandle:logical_width()
    return self:props().logical_width
end

---Get this output's logical height in pixels.
---
---If the output is disabled, this returns nil.
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

---Get this output's transform.
---
---Shorthand for `handle:props().transform`.
---
---@return Transform?
function OutputHandle:transform()
    return self:props().transform
end

---Get this output's EDID serial number.
---
---Shorthand for `handle:props().serial`.
---
---@return integer?
function OutputHandle:serial()
    return self:props().serial
end

---Get this output's keyboard focus stack.
---
---This includes *all* windows on the output, even those on inactive tags.
---If you only want visible windows, use `keyboard_focus_stack_visible` instead.
---
---Shorthand for `handle:props().keyboard_focus_stack`.
---
---@return WindowHandle[]
---
---@see OutputHandle.keyboard_focus_stack_visible
function OutputHandle:keyboard_focus_stack()
    return self:props().keyboard_focus_stack
end

---Get whether this output is enabled.
---
---Disabled outputs are not mapped to the global space and cannot be used.
---
---@return boolean?
function OutputHandle:enabled()
    return self:props().enabled
end

---Get whether this output is powered.
---
---Unpowered outputs that are enabled will be off, but they will still be
---mapped to the global space, meaning you can still interact with them.
---
---@return boolean?
function OutputHandle:powered()
    return self:props().powered
end

---Get this output's keyboard focus stack.
---
---This only includes windows on active tags.
---If you want all windows on the output, use `keyboard_focus_stack` instead.
---
---@return WindowHandle[]
---
---@see OutputHandle.keyboard_focus_stack
function OutputHandle:keyboard_focus_stack_visible()
    local stack = self:props().keyboard_focus_stack

    ---@type (fun(): boolean)[]
    local batch = {}
    for i, win in ipairs(stack) do
        batch[i] = function()
            return win:is_on_active_tag()
        end
    end

    local on_active_tags = require("pinnacle.util").batch(batch)

    ---@type WindowHandle[]
    local keyboard_focus_stack_visible = {}

    for i, is_active in ipairs(on_active_tags) do
        if is_active then
            table.insert(keyboard_focus_stack_visible, stack[i])
        end
    end

    return keyboard_focus_stack_visible
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
