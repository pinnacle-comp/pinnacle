-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---Tag management
---@module TagModule
local tag_module = {}

---@alias Layout
---| "MasterStack" # One master window on the left with all other windows stacked to the right.
---| "Dwindle" # Windows split in half towards the bottom right corner.
---| "Spiral" # Windows split in half in a spiral.
---| "CornerTopLeft" # One main corner window in the top left with a column of windows on the right and a row on the bottom.
---| "CornerTopRight" # One main corner window in the top right with a column of windows on the left and a row on the bottom.
---| "CornerBottomLeft" # One main corner window in the bottom left with a column of windows on the right and a row on the top.
---| "CornerBottomRight" # One main corner window in the bottom right with a column of windows on the left and a row on the top.

---Add tags to the specified output.
---
---### Examples
---    local op = output.get_by_name("DP-1")
---    if op ~= nil then
---        tag.add(op, "1", "2", "3", "4", "5") -- Add tags with names 1-5
---    end
---You can also pass in a table.
---    local tags = {"Terminal", "Browser", "Code", "Potato", "Email"}
---    tag.add(op, tags) -- Add tags with those names
---@tparam Output output The output you want these tags to be added to.
---@tparam string ... The names of the new tags you want to add.
---@see Output.add_tags
function tag_module.add(output, ...) end

---Toggle a tag on the specified output. If `output` isn't specified, toggle it on the currently focused output instead.
---
---### Example
---    -- Assuming all tags are toggled off...
---    local op = output.get_by_name("DP-1")
---    tag.toggle("1", op)
---    tag.toggle("2", op)
---    -- will cause windows on both tags 1 and 2 to be displayed at the same time.
---@tparam string name The name of the tag.
---@tparam ?Output output The output.
---@see Tag.toggle
function tag_module.toggle(name, output) end

---Switch to a tag on the specified output, deactivating any other active tags on it.
---If `output` is not specified, this uses the currently focused output instead.
---Alternatively, provide a tag object instead of a name and output.
---
---This is used to replicate what a traditional workspace is on some other Wayland compositors.
---
---### Examples
---    -- Switches to and displays *only* windows on tag `3` on the focused output.
---    tag.switch_to("3")
---@tparam string name The name of the tag.
---@tparam ?Output output The output.
---@see Tag.switch_to
function tag_module.switch_to(name, output) end

---Set a layout for the tag on the specified output. If no output is provided, set it for the tag on the currently focused one.
---Alternatively, provide a tag object instead of a name and output.
---
---### Examples
---    -- Set tag `1` on `DP-1` to the `Dwindle` layout
---    tag.set_layout("1", "Dwindle", output.get_by_name("DP-1"))
---
---    -- Do the same as above. Note: if you have more than one tag named `1` then this picks the first one.
---    local t = tag.get_by_name("1")[1]
---    tag.set_layout(t, "Dwindle")
---@tparam string name The name of the tag.
---@tparam Layout layout The layout.
---@tparam ?Output output The output.
---@see Tag.set_layout
function tag_module.set_layout(name, layout, output) end

---Get all tags on the specified output.
---
---### Example
---    local op = output.get_focused()
---    if op ~= nil then
---        local tags = tag.get_on_output(op) -- All tags on the focused output
---    end
---@tparam Output output
---@treturn Tag[]
---@see Output.tags
function tag_module.get_on_output(output) end

---Get all tags with this name across all outputs.
---
---### Example
---    -- Given one monitor with the tags "OBS", "OBS", "VSCode", and "Spotify"...
---    local tags = tag.get_by_name("OBS")
---    -- ...will have 2 tags in `tags`, while...
---    local no_tags = tag.get_by_name("Firefox")
---    -- ...will have `no_tags` be empty.
---@tparam string name The name of the tag(s) you want.
---@treturn Tag[]
function tag_module.get_by_name(name) end

---Get all tags across all outputs.
---
---### Example
---    -- With two monitors with the same tags: "1", "2", "3", "4", and "5"...
---    local tags = tag.get_all()
---    -- ...`tags` should have 10 tags, with 5 pairs of those names across both outputs.
---@treturn Tag[]
function tag_module.get_all() end

---Get the specified tag's name.
---
---### Example
---    -- Assuming the tag `Terminal` exists...
---    print(tag.name(tag.get_by_name("Terminal")[1]))
---    -- ...should print `Terminal`.
---@tparam Tag t
---@treturn string|nil
---@see Tag.name
function tag_module.name(t) end

---Get whether or not the specified tag is active.
---@tparam Tag t
---@treturn boolean|nil
---@see Tag.active
function tag_module.active(t) end

---Get the output the specified tag is on.
---@tparam Tag t
---@treturn Output|nil
---@see OutputModule.get_for_tag
---@see Tag.output
function tag_module.output(t) end

-----------------------------------------------------

---Tag objects
---@classmod Tag
local tag = {}

---Get this tag's active status.
---@treturn boolean|nil active `true` if the tag is active, `false` if not, and `nil` if the tag doesn't exist.
---@see TagModule.active
function tag:active() end

---Get this tag's name.
---@treturn string|nil name The name of this tag, or nil if it doesn't exist.
---@see TagModule.name
function tag:name() end

---Get this tag's output.
---@treturn Output|nil output The output this tag is on, or nil if the tag doesn't exist.
---@see TagModule.output
function tag:output() end

---Switch to this tag.
---@see TagModule.switch_to
function tag:switch_to() end

---Toggle this tag.
---@see TagModule.toggle
function tag:toggle() end

---Set this tag's layout.
---@tparam Layout layout
---@see TagModule.set_layout
function tag:set_layout(layout) end
