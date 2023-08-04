-- SPDX-License-Identifier: GPL-3.0-or-later

---Output management
---@module OutputModule
local output_module = {}

---Get an output by its name.
---
---"Name" in this sense does not mean its model or manufacturer;
---rather, "name" is the name of the connector the output is connected to.
---This should be something like "HDMI-A-0", "eDP-1", or similar.
---
---@usage
---local monitor = output.get_by_name("DP-1")
---print(monitor.name) -- should print `DP-1`
---@tparam string name The name of the output.
---@treturn Output|nil output The output, or nil if none have the provided name.
function output_module.get_by_name(name) end

---Note: This may or may not be what is reported by other monitor listing utilities. Pinnacle currently fails to pick up one of my monitors' models when it is correctly picked up by tools like wlr-randr. I'll fix this in the future.
---
---Get outputs by their model.
---This is something like "DELL E2416H" or whatever gibberish monitor manufacturers call their displays.
---@tparam string model The model of the output(s).
---@treturn Output[] outputs All outputs with this model.
function output_module.get_by_model(model) end

---Get outputs by their resolution.
---
---@tparam integer width The width of the outputs, in pixels.
---@tparam integer height The height of the outputs, in pixels.
---@treturn Output[] outputs All outputs with this resolution.
function output_module.get_by_res(width, height) end

---Get the currently focused output. This is currently implemented as the one with the cursor on it.
---
---This function may return nil, which means you may get a warning if you try to use it without checking for nil.
---Usually this function will not be nil unless you unplug all monitors, so instead of checking,
---you can ignore the warning by either forcing the type to be non-nil with an inline comment:
---    local op = output.get_focused() --[[@as Output]]
---or by disabling nil check warnings for the line:
---    local op = output.get_focused()
---    ---@diagnostic disable-next-line:need-check-nil
---    local tags_on_output = op:tags()
---Type checking done by Lua LS isn't perfect.
---Note that directly using the result of this function inline will *not* raise a warning, so be careful.
---    local tags = output.get_focused():tags() -- will NOT warn for nil
---@treturn Output|nil output The output, or nil if none are focused.
function output_module.get_focused() end

---Connect a function to be run on all current and future outputs.
---
---When called, `connect_for_all` will immediately run `func` with all currently connected outputs.
---If a new output is connected, `func` will also be called with it.
---
---Please note: this function will be run *after* Pinnacle processes your entire config.
---For example, if you define tags in `func` but toggle them directly after `connect_for_all`, nothing will happen as the tags haven't been added yet.
---@tparam function func The function that will be run. This takes an `Output` object.
function output_module.connect_for_all(func) end

---Get the output the specified tag is on.
---@tparam Tag tag
---@treturn Output|nil
---@see TagModule.output
---@see Tag.output
function output_module.get_for_tag(tag) end

---Get the specified output's make.
---@tparam Output op
---@treturn string|nil
---@see Output.make
function output_module.make(op) end

---Get the specified output's model.
---@tparam Output op
---@treturn string|nil
---@see Output.model
function output_module.model(op) end

---Get the specified output's location in the global space, in pixels.
---@tparam Output op
---@treturn table|nil { x: integer, y: integer }
---@see Output.loc
function output_module.loc(op) end

---Get the specified output's resolution in pixels.
---@tparam Output op
---@treturn table|nil { w: integer, h: integer }
---@see Output.res
function output_module.res(op) end

---Get the specified output's refresh rate in millihertz.
---For example, 60Hz will be returned as 60000.
---@tparam Output op
---@treturn integer|nil
---@see Output.refresh_rate
function output_module.refresh_rate(op) end

---Get the specified output's physical size in millimeters.
---@tparam Output op
---@treturn table|nil { w: integer, h: integer }
---@see Output.physical_size
function output_module.physical_size(op) end

---Get whether or not the specified output is focused. This is currently defined as having the cursor on it.
---@tparam Output op
---@treturn boolean|nil
---@see Output.focused
function output_module.focused(op) end

---Get the specified output's tags.
---@tparam Output op
---@see TagModule.get_on_output
---@see Output.tags
function output_module.tags(op) end

---Add tags to the specified output.
---@tparam Output op
---@tparam string ... The names of the tags you want to add. You can also pass in a table.
---@see TagModule.add
---@see Output.add_tags
function output_module.add_tags(op, ...) end

---Set the specified output's location.
---
---@usage
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
---@tparam Output op
---@tparam table loc A table of the form `{ x: integer?, y: integer? }`
function output_module.set_loc(op, loc) end

----------------------------------------------------------

---The output object.
---@classmod Output
local output = {}

---Get this output's name. This is something like "eDP-1" or "HDMI-A-0".
---@treturn string
function output:name() end

---Get all tags on this output.
---@treturn Tag[]
---@see OutputModule.tags
function output:tags() end

---Add tags to this output.
---@tparam string ... The names of the tags you want to add. You can also pass in a table.
---@see OutputModule.add_tags
function output:add_tags(...) end

---Get this output's make.
---@treturn string|nil
---@see OutputModule.make
function output:make() end

---Get this output's model.
---@treturn string|nil
---@see OutputModule.model
function output:model() end

---Get this output's location in the global space, in pixels.
---@treturn table|nil { x: integer, y: integer }
---@see OutputModule.loc
function output:loc() end

---Get this output's resolution in pixels.
---@treturn table|nil { w: integer, h: integer }
---@see OutputModule.res
function output:res() end

---Get this output's refresh rate in millihertz.
---For example, 60Hz will be returned as 60000.
---@treturn integer|nil
---@see OutputModule.refresh_rate
function output:refresh_rate() end

---Get this output's physical size in millimeters.
---@treturn table|nil { w: integer, h: integer }
---@see OutputModule.physical_size
function output:physical_size() end

---Get whether or not this output is focused. This is currently defined as having the cursor on it.
---@treturn boolean|nil
---@see OutputModule.focused
function output:focused() end

---Set this output's location.
---
---@usage
--- -- Assuming DP-1 is 2560x1440 and DP-2 is 1920x1080...
---local dp1 = output.get_by_name("DP-1")
---local dp2 = output.get_by_name("DP-2")
---
--- -- Place DP-2 to the left of DP-1, top borders aligned
---dp1:set_loc({ x = 1920, y = 0 })
---dp2:set_loc({ x = 0, y = 0 })
---
--- -- Do the same as above, with a different origin
---dp1:set_loc({ x = 0, y = 0 })
---dp2:set_loc({ x = -1920, y = 0 })
---
--- -- Place DP-2 to the right of DP-1, bottom borders aligned
---dp1:set_loc({ x = 0, y = 0 })
---dp2:set_loc({ x = 2560, y = 1440 - 1080 })
---@tparam table loc A table of the form `{ x: integer?, y: integer? }`
function output:set_loc(loc) end

---Set this output's location to the right of `op`.
---
---This will fail if `op` is an invalid output.
---@tparam Output op
---@tparam[opt="top"] string alignment One of `top`, `center`, or `bottom`. This is how you want to align the `self` output.
---@see Output.set_loc
function output:set_loc_right_of(op, alignment) end

---Set this output's location to the left of `op`.
---
---This will fail if `op` is an invalid output.
---@tparam Output op
---@tparam[opt="top"] string alignment One of `top`, `center`, or `bottom`. This is how you want to align the `self` output.
---@see Output.set_loc
function output:set_loc_left_of(op, alignment) end

---Set this output's location to the top of `op`.
---
---This will fail if `op` is an invalid output.
---@tparam Output op
---@tparam[opt="left"] string alignment One of `left`, `center`, or `right`. This is how you want to align the `self` output.
---@see Output.set_loc
function output:set_loc_top_of(op, alignment) end

---Set this output's location to the bottom of `op`.
---
---This will fail if `op` is an invalid output.
---@tparam Output op
---@tparam[opt="left"] string alignment One of `left`, `center`, or `right`. This is how you want to align the `self` output.
---@see Output.set_loc
function output:set_loc_bottom_of(op, alignment) end
