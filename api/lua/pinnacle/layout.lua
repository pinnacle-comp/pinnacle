-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local protobuf = require("pinnacle.grpc.protobuf")
local layout_service = require("pinnacle.grpc.defs").pinnacle.layout.v0alpha1.LayoutService

local mfloor = math.floor

---@class LayoutArgs
---@field output OutputHandle
---@field windows WindowHandle[]
---@field tags TagHandle[]
---@field output_width integer
---@field output_height integer

---A layout generator.
---@class LayoutGenerator
---Generate an array of geometries from the given `LayoutArgs`.
---@field layout fun(self: self, args: LayoutArgs): { x: integer, y: integer, width: integer, height: integer }[]

---Builtin layout generators.
---
---This contains functions that create various builtin generators.
---@class Builtin
local builtins = {}

----------------------------------------
-- Master Stack                       --
----------------------------------------

---A `LayoutGenerator` that has one master area to one side
---and a stack of windows next to it.
---@class Builtin.MasterStack: LayoutGenerator
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---@field gaps integer | { inner: integer, outer: integer }
---The proportion of the output taken up by the master window(s).
---
---This is a float that will be clamped between 0.1 and 0.9.
---@field master_factor number
---The side the master window(s) will be on.
---@field master_side "left"|"right"|"top"|"bottom"
---How many windows the master side will have.
---@field master_count integer
local MasterStack = {
    gaps = 8,
    master_factor = 0.5,
    master_side = "left",
    master_count = 1,
}

---@class Builtin.MasterStack.Args
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 8.
---@field gaps? integer | { inner: integer, outer: integer }
---The proportion of the output taken up by the master window(s).
---
---This is a float that will be clamped between 0.1 and 0.9.
---
---Defaults to 0.5.
---@field master_factor? number
---The side the master window(s) will be on.
---
---Defaults to "left".
---@field master_side? "left"|"right"|"top"|"bottom"
---How many windows the master side will have.
---
---Defaults to 1.
---@field master_count? integer

---@param args LayoutArgs
---@return { x: integer, y: integer, width: integer, height: integer }[]
function MasterStack:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    ---@type { x: integer, y: integer, width: integer, height: integer }[]
    local geos = {}

    ---@type integer
    local outer_gaps
    ---@type integer?
    local inner_gaps

    if type(self.gaps) == "number" then
        outer_gaps = self.gaps --[[@as integer]]
    else
        outer_gaps = self.gaps.outer
        inner_gaps = self.gaps.inner
    end

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    rect = rect:split_at("horizontal", 0, outer_gaps)
    rect = rect:split_at("horizontal", height - outer_gaps, outer_gaps)
    rect = rect:split_at("vertical", 0, outer_gaps)
    rect = rect:split_at("vertical", width - outer_gaps, outer_gaps)

    local master_factor = math.max(math.min(self.master_factor, 0.9), 0.1)
    if win_count <= self.master_count then
        master_factor = 1
    end

    local master_rect
    local stack_rect

    local gaps = ((not inner_gaps and outer_gaps) or 0)

    if self.master_side == "left" then
        master_rect, stack_rect =
            rect:split_at("vertical", mfloor(width * master_factor) - mfloor(gaps / 2), gaps)
    elseif self.master_side == "right" then
        stack_rect, master_rect =
            rect:split_at("vertical", mfloor(width * master_factor) - mfloor(gaps / 2), gaps)
    elseif self.master_side == "top" then
        master_rect, stack_rect =
            rect:split_at("horizontal", mfloor(height * master_factor) - mfloor(gaps / 2), gaps)
    else
        stack_rect, master_rect =
            rect:split_at("horizontal", mfloor(height * master_factor) - mfloor(gaps / 2), gaps)
    end

    if not master_rect then
        assert(stack_rect)
        master_rect = stack_rect
        stack_rect = nil
    end

    local master_slice_count
    local stack_slice_count = nil

    if win_count > self.master_count then
        master_slice_count = self.master_count - 1
        stack_slice_count = win_count - self.master_count - 1
    else
        master_slice_count = win_count - 1
    end

    -- layout the master side
    if master_slice_count > 0 then
        local coord
        local len
        local axis

        if self.master_side == "left" or self.master_side == "right" then
            coord = master_rect.y
            len = mfloor(master_rect.height / (master_slice_count + 1))
            axis = "horizontal"
        else
            coord = master_rect.x
            len = mfloor(master_rect.width / (master_slice_count + 1))
            axis = "vertical"
        end

        for i = 1, master_slice_count do
            local slice_point = coord + mfloor(len * i + 0.5)
            slice_point = slice_point - mfloor(gaps / 2)
            local to_push, rest = master_rect:split_at(axis, slice_point, gaps)
            table.insert(geos, to_push)
            master_rect = rest
        end
    end

    table.insert(geos, master_rect)

    if stack_slice_count then
        assert(stack_rect)

        if stack_slice_count > 0 then
            local coord
            local len
            local axis
            if self.master_side == "left" or self.master_side == "right" then
                coord = stack_rect.y
                len = stack_rect.height / (stack_slice_count + 1)
                axis = "horizontal"
            else
                coord = stack_rect.x
                len = stack_rect.width / (stack_slice_count + 1)
                axis = "vertical"
            end

            for i = 1, stack_slice_count do
                local slice_point = coord + mfloor(len * i + 0.5)
                slice_point = slice_point - mfloor(gaps / 2)
                local to_push, rest = stack_rect:split_at(axis, slice_point, gaps)
                table.insert(geos, to_push)
                stack_rect = rest
            end
        end

        table.insert(geos, stack_rect)
    end

    if inner_gaps then
        for i = 1, #geos do
            geos[i].x = geos[i].x + inner_gaps
            geos[i].y = geos[i].y + inner_gaps
            geos[i].width = geos[i].width - inner_gaps * 2
            geos[i].height = geos[i].height - inner_gaps * 2
        end
    end

    return geos
end

---Create a master stack layout generator.
---
---Pass in `settings` to override the defaults.
---
---@param settings? Builtin.MasterStack.Args
---
---@return Builtin.MasterStack
function builtins.master_stack(settings)
    local master_stack = settings or {}
    setmetatable(master_stack, { __index = MasterStack })
    ---@cast master_stack Builtin.MasterStack
    return master_stack
end

----------------------------------------
-- Dwindle                            --
----------------------------------------

---A `LayoutGenerator` that lays out windows in a shrinking fashion
---towards the bottom right corner.
---@class Builtin.Dwindle: LayoutGenerator
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---@field gaps integer | { inner: integer, outer: integer }
---The proportions that each split will split at.
---
---The first split will use the factor at [1],
---the second at [2], and so on.
---@field split_factors table<integer, number>
local Dwindle = {
    gaps = 8,
    split_factors = {},
}

---@class Builtin.Dwindle.Args
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 8.
---@field gaps? integer | { inner: integer, outer: integer }
---The proportions that each split will split at.
---
---The first split will use the factor at [1],
---the second at [2], and so on.
---
---Defaults to 0.5 if there is no factor at [n].
---@field split_factors? table<integer, number>

---@param args LayoutArgs
---
---@return { x: integer, y: integer, width: integer, height: integer }[]
function Dwindle:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    ---@type Rectangle[]
    local geos = {}

    ---@type integer
    local outer_gaps
    ---@type integer?
    local inner_gaps

    if type(self.gaps) == "number" then
        outer_gaps = self.gaps --[[@as integer]]
    else
        outer_gaps = self.gaps.outer
        inner_gaps = self.gaps.inner
    end

    rect = rect:split_at("horizontal", 0, outer_gaps)
    rect = rect:split_at("horizontal", height - outer_gaps, outer_gaps)
    rect = rect:split_at("vertical", 0, outer_gaps)
    rect = rect:split_at("vertical", width - outer_gaps, outer_gaps)

    if win_count == 1 then
        table.insert(geos, rect)
    else
        local gaps = ((not inner_gaps and outer_gaps) or 0)

        ---@type Rectangle
        local rest = rect

        for i = 1, win_count - 1 do
            local factor = math.min(math.max(self.split_factors[i] or 0.5, 0.1), 0.9)
            local axis
            local split_coord
            if i % 2 == 1 then
                axis = "vertical"
                split_coord = rest.x + mfloor(rest.width * factor + 0.5)
            else
                axis = "horizontal"
                split_coord = rest.y + mfloor(rest.height * factor + 0.5)
            end
            split_coord = split_coord - mfloor(gaps / 2)

            local to_push

            ---@diagnostic disable-next-line: cast-local-type
            to_push, rest = rest:split_at(axis, split_coord, gaps)

            if not rest then
                break
            end

            table.insert(geos, to_push)
        end

        table.insert(geos, rest)
    end

    if inner_gaps then
        for i = 1, #geos do
            geos[i].x = geos[i].x + inner_gaps
            geos[i].y = geos[i].y + inner_gaps
            geos[i].width = geos[i].width - inner_gaps * 2
            geos[i].height = geos[i].height - inner_gaps * 2
        end
    end

    return geos
end

---Create a dwindle layout generator.
---
---Pass in `settings` to override the defaults.
---
---@param settings? Builtin.Dwindle.Args
---
---@return Builtin.Dwindle
function builtins.dwindle(settings)
    local dwindle = settings or {}
    setmetatable(dwindle, { __index = Dwindle })
    ---@cast dwindle Builtin.Dwindle
    return dwindle
end

----------------------------------------
-- Corner                             --
----------------------------------------

---A `LayoutGenerator` that has one main corner window and a
---horizontal and vertical stack flanking it on the other two sides.
---@class Builtin.Corner: LayoutGenerator
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---@field gaps integer | { inner: integer, outer: integer }
---How much of the output the corner window's width will take up.
---@field corner_width_factor number
---How much of the output the corner window's height will take up.
---@field corner_height_factor number
---Which corner the corner window will be in.
---@field corner_loc "top_left"|"top_right"|"bottom_left"|"bottom_right"
local Corner = {
    gaps = 8,
    corner_width_factor = 0.5,
    corner_height_factor = 0.5,
    corner_loc = "top_left",
}

---@class Builtin.Corner.Args
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 8.
---@field gaps? integer | { inner: integer, outer: integer }
---How much of the output the corner window's width will take up.
---
---Defaults to 0.5.
---@field corner_width_factor? number
---How much of the output the corner window's height will take up.
---
---Defaults to 0.5.
---@field corner_height_factor? number
---Which corner the corner window will be in.
---
---Defaults to "top_left".
---@field corner_loc? "top_left"|"top_right"|"bottom_left"|"bottom_right"

---@param args LayoutArgs
---
---@return { x: integer, y: integer, width: integer, height: integer }[]
function Corner:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    ---@type Rectangle[]
    local geos = {}

    ---@type integer
    local outer_gaps
    ---@type integer?
    local inner_gaps

    if type(self.gaps) == "number" then
        outer_gaps = self.gaps --[[@as integer]]
    else
        outer_gaps = self.gaps.outer
        inner_gaps = self.gaps.inner
    end

    rect = rect:split_at("horizontal", 0, outer_gaps)
    rect = rect:split_at("horizontal", height - outer_gaps, outer_gaps)
    rect = rect:split_at("vertical", 0, outer_gaps)
    rect = rect:split_at("vertical", width - outer_gaps, outer_gaps)

    if win_count == 1 then
        table.insert(geos, rect)
    else
        local gaps = ((not inner_gaps and outer_gaps) or 0)

        local corner_rect, vert_stack_rect

        if self.corner_loc == "top_left" or self.corner_loc == "bottom_left" then
            local x_slice_point = rect.x
                + mfloor(rect.width * self.corner_width_factor + 0.5)
                - mfloor(gaps / 2)
            corner_rect, vert_stack_rect = rect:split_at("vertical", x_slice_point, gaps)
        else
            local x_slice_point = rect.x
                + mfloor(rect.width * (1 - self.corner_width_factor) + 0.5)
                - mfloor(gaps / 2)
            vert_stack_rect, corner_rect = rect:split_at("vertical", x_slice_point, gaps)
        end

        if win_count == 2 then
            table.insert(geos, corner_rect)
            table.insert(geos, vert_stack_rect)
        else
            assert(corner_rect)

            local horiz_stack_rect

            if self.corner_loc == "top_left" or self.corner_loc == "top_right" then
                local y_slice_point = rect.y
                    + mfloor(rect.height * self.corner_height_factor + 0.5)
                    - mfloor(gaps / 2)
                corner_rect, horiz_stack_rect =
                    corner_rect:split_at("horizontal", y_slice_point, gaps)
            else
                local y_slice_point = rect.y
                    + mfloor(rect.height * (1 - self.corner_height_factor) + 0.5)
                    - mfloor(gaps / 2)
                horiz_stack_rect, corner_rect =
                    corner_rect:split_at("horizontal", y_slice_point, gaps)
            end

            assert(horiz_stack_rect)
            assert(vert_stack_rect)
            assert(corner_rect)

            table.insert(geos, corner_rect)

            -- win_count >= 3 here

            ---@type Rectangle[]
            local vert_geos = {}
            ---@type Rectangle[]
            local horiz_geos = {}

            local vert_stack_count = math.ceil((win_count - 1) / 2)
            local horiz_stack_count = mfloor((win_count - 1) / 2)

            local vert_stack_y = vert_stack_rect.y
            local vert_win_height = vert_stack_rect.height / vert_stack_count

            for i = 1, vert_stack_count - 1 do
                local slice_point = vert_stack_y + mfloor(vert_win_height * i + 0.5)
                slice_point = slice_point - mfloor(gaps / 2)
                local to_push, rest = vert_stack_rect:split_at("horizontal", slice_point, gaps)
                table.insert(vert_geos, to_push)
                vert_stack_rect = rest
            end

            table.insert(vert_geos, vert_stack_rect)

            local horiz_stack_x = horiz_stack_rect.x
            local horiz_win_width = horiz_stack_rect.width / horiz_stack_count

            for i = 1, horiz_stack_count - 1 do
                local slice_point = horiz_stack_x + mfloor(horiz_win_width * i + 0.5)
                slice_point = slice_point - mfloor(gaps / 2)
                local to_push, rest = horiz_stack_rect:split_at("vertical", slice_point, gaps)
                table.insert(horiz_geos, to_push)
                horiz_stack_rect = rest
            end

            table.insert(horiz_geos, horiz_stack_rect)

            -- Alternate between the vertical and horizontal stacks
            for i = 1, #vert_geos + #horiz_geos do
                if i % 2 == 1 then
                    table.insert(geos, vert_geos[math.ceil(i / 2)])
                else
                    table.insert(geos, horiz_geos[i / 2])
                end
            end
        end
    end

    if inner_gaps then
        for i = 1, #geos do
            geos[i].x = geos[i].x + inner_gaps
            geos[i].y = geos[i].y + inner_gaps
            geos[i].width = geos[i].width - inner_gaps * 2
            geos[i].height = geos[i].height - inner_gaps * 2
        end
    end

    return geos
end

---Create a corner layout generator.
---
---Pass in `settings` to override the defaults.
---
---@param settings? Builtin.Corner.Args
---
---@return Builtin.Corner
function builtins.corner(settings)
    local corner = settings or {}
    setmetatable(corner, { __index = Corner })
    ---@cast corner Builtin.Corner
    return corner
end

----------------------------------------
-- Spiral                             --
----------------------------------------

---A `LayoutGenerator` that lays out windows in a spiral.
---
---This is similar to the dwindle layout but in a spiral instead of
---towards the borrom right corner.
---@class Builtin.Spiral : LayoutGenerator
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---@field gaps integer | { inner: integer, outer: integer }
---The proportions that each split will split at.
---
---The first split will use the factor at [1],
---the second at [2], and so on.
---@field split_factors table<integer, number>
local Spiral = {
    gaps = 8,
    split_factors = {},
}

---@class Builtin.Spiral.Args
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 8.
---@field gaps? integer | { inner: integer, outer: integer }
---The proportions that each split will split at.
---
---The first split will use the factor at [1],
---the second at [2], and so on.
---
---Defaults to 0.5 if there is no factor at [n].
---@field split_factors? table<integer, number>

---@param args LayoutArgs
---
---@return { x: integer, y: integer, width: integer, height: integer }[]
function Spiral:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    ---@type Rectangle[]
    local geos = {}

    ---@type integer
    local outer_gaps
    ---@type integer?
    local inner_gaps

    if type(self.gaps) == "number" then
        outer_gaps = self.gaps --[[@as integer]]
    else
        outer_gaps = self.gaps.outer
        inner_gaps = self.gaps.inner
    end

    rect = rect:split_at("horizontal", 0, outer_gaps)
    rect = rect:split_at("horizontal", height - outer_gaps, outer_gaps)
    rect = rect:split_at("vertical", 0, outer_gaps)
    rect = rect:split_at("vertical", width - outer_gaps, outer_gaps)

    if win_count == 1 then
        table.insert(geos, rect)
    else
        local gaps = ((not inner_gaps and outer_gaps) or 0)

        ---@type Rectangle
        local rest = rect

        for i = 1, win_count - 1 do
            local factor = math.min(math.max(self.split_factors[i] or 0.5, 0.1), 0.9)
            local axis
            local split_coord
            if i % 2 == 1 then
                axis = "vertical"
                split_coord = rest.x + mfloor(rest.width * factor + 0.5)
            else
                axis = "horizontal"
                split_coord = rest.y + mfloor(rest.height * factor + 0.5)
            end
            split_coord = split_coord - mfloor(gaps / 2)

            local to_push

            -- Minor change from dwindle here
            if i % 4 == 3 or i % 4 == 0 then
                rest, to_push = rest:split_at(axis, split_coord, gaps)
            else
                ---@diagnostic disable-next-line: cast-local-type
                to_push, rest = rest:split_at(axis, split_coord, gaps)
            end

            if not rest then
                break
            end

            table.insert(geos, to_push)
        end

        table.insert(geos, rest)
    end

    if inner_gaps then
        for i = 1, #geos do
            geos[i].x = geos[i].x + inner_gaps
            geos[i].y = geos[i].y + inner_gaps
            geos[i].width = geos[i].width - inner_gaps * 2
            geos[i].height = geos[i].height - inner_gaps * 2
        end
    end

    return geos
end

---Create a spiral layout generator.
---
---Pass in `settings` to override the defaults.
---
---@param settings? Builtin.Spiral.Args
---
---@return Builtin.Spiral
function builtins.spiral(settings)
    local spiral = settings or {}
    setmetatable(spiral, { __index = Spiral })
    ---@cast spiral Builtin.Spiral
    return spiral
end

----------------------------------------
-- Fair                               --
----------------------------------------

---A `LayoutGenerator` that attempts to layout windows such that
---they are the same size.
---@class Builtin.Fair : LayoutGenerator
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---@field gaps integer | { inner: integer, outer: integer }
---The direction of the lines of windows.
---@field direction "horizontal"|"vertical"
local Fair = {
    gaps = 8,
    direction = "vertical",
}

---@class Builtin.Fair.Args
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 8.
---@field gaps? integer | { inner: integer, outer: integer }
---The direction of the lines of windows.
---
---Defaults to "vertical".
---@field direction? "horizontal"|"vertical"

---@param args LayoutArgs
---
---@return { x: integer, y: integer, width: integer, height: integer }[]
function Fair:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    ---@type Rectangle[]
    local geos = {}

    ---@type integer
    local outer_gaps
    ---@type integer?
    local inner_gaps

    if type(self.gaps) == "number" then
        outer_gaps = self.gaps --[[@as integer]]
    else
        outer_gaps = self.gaps.outer
        inner_gaps = self.gaps.inner
    end

    rect = rect:split_at("horizontal", 0, outer_gaps)
    rect = rect:split_at("horizontal", height - outer_gaps, outer_gaps)
    rect = rect:split_at("vertical", 0, outer_gaps)
    rect = rect:split_at("vertical", width - outer_gaps, outer_gaps)

    local gaps = ((not inner_gaps and outer_gaps) or 0)

    if win_count == 1 then
        table.insert(geos, rect)
    elseif win_count == 2 then
        local len
        local coord
        if self.direction == "vertical" then
            len = rect.width
            coord = rect.x
        else
            len = rect.height
            coord = rect.y
        end
        -- Two windows is special cased to create a new line rather than increase to 2 in a line
        local rect1, rect2 =
            rect:split_at(self.direction, coord + mfloor(len / 2) - mfloor(gaps / 2), gaps)
        if rect1 and rect2 then
            table.insert(geos, rect1)
            table.insert(geos, rect2)
        end
    else
        local line_count = mfloor(math.sqrt(win_count) + 0.5)
        local wins_per_line = {}
        local max_per_line = line_count
        if win_count > line_count * line_count then
            max_per_line = line_count + 1
        end
        for i = 1, win_count do
            local index = math.ceil(i / max_per_line)
            if not wins_per_line[index] then
                wins_per_line[index] = 0
            end
            wins_per_line[index] = wins_per_line[index] + 1
        end

        assert(#wins_per_line == line_count)

        ---@type Rectangle[]
        local line_rects = {}

        local coord
        local len
        local axis
        if self.direction == "horizontal" then
            coord = rect.y
            len = rect.height / line_count
            axis = "horizontal"
        else
            coord = rect.x
            len = rect.width / line_count
            axis = "vertical"
        end

        for i = 1, line_count - 1 do
            local slice_point = coord + mfloor(len * i + 0.5)
            slice_point = slice_point - mfloor(gaps / 2)
            local to_push, rest = rect:split_at(axis, slice_point, gaps)
            table.insert(line_rects, to_push)
            if not rest then
                break
            end
            rect = rest
        end

        table.insert(line_rects, rect)

        for i, line_rect in ipairs(line_rects) do
            local coord ---@diagnostic disable-line: redefined-local
            local len ---@diagnostic disable-line: redefined-local
            local axis ---@diagnostic disable-line: redefined-local
            if self.direction == "vertical" then
                coord = line_rect.y
                len = line_rect.height / wins_per_line[i]
                axis = "horizontal"
            else
                coord = line_rect.x
                len = line_rect.width / wins_per_line[i]
                axis = "vertical"
            end

            for j = 1, wins_per_line[i] - 1 do
                local slice_point = coord + mfloor(len * j + 0.5)
                slice_point = slice_point - mfloor(gaps / 2)
                local to_push, rest = line_rect:split_at(axis, slice_point, gaps)
                table.insert(geos, to_push)
                if not rest then
                    break
                end
                line_rect = rest
            end

            table.insert(geos, line_rect)
        end
    end

    if inner_gaps then
        for i = 1, #geos do
            geos[i].x = geos[i].x + inner_gaps
            geos[i].y = geos[i].y + inner_gaps
            geos[i].width = geos[i].width - inner_gaps * 2
            geos[i].height = geos[i].height - inner_gaps * 2
        end
    end

    return geos
end

---Create a fair layout generator.
---
---Pass in `settings` to override the defaults.
---
---@param settings? Builtin.Fair.Args
---
---@return Builtin.Fair
function builtins.fair(settings)
    local fair = settings or {}
    setmetatable(fair, { __index = Fair })
    ---@cast fair Builtin.Fair
    return fair
end

---@class Layout
---@field private stream H2Stream?
local layout = {
    builtins = builtins,
}

---Set the layout manager for this config.
---
---It will manage layout requests from the compositor.
---
---Only one layout manager can manage layouts at a time.
---
---@param manager LayoutManager
function layout.set_manager(manager)
    layout.stream = client():bidirectional_streaming_request(layout_service.Layout, {
        layout = {},
    }, function(response, stream)
        local request_id = response.request_id

        ---@diagnostic disable-next-line: invisible
        local output_handle = require("pinnacle.output").handle.new(response.output_name)

        local window_handles =
            ---@diagnostic disable-next-line: invisible
            require("pinnacle.window").handle.new_from_table(response.window_ids or {})

        ---@diagnostic disable-next-line: invisible
        local tag_handles = require("pinnacle.tag").handle.new_from_table(response.tag_ids or {})

        ---@type LayoutArgs
        local args = {
            output = output_handle,
            windows = window_handles,
            tags = tag_handles,
            output_width = response.output_width,
            output_height = response.output_height,
        }

        local a = manager:get_active(args)
        local success, geos = pcall(a.layout, a, args)

        if not success then
            print(geos)
            os.exit(1)
        end

        local body = protobuf.encode(".pinnacle.layout.v0alpha1.LayoutRequest", {
            geometries = {
                request_id = request_id,
                geometries = geos,
                output_name = response.output_name,
            },
        })

        stream:write_chunk(body, false)
    end)
end

---Request a layout on the given output, or the focused output if nil.
---
---If no `LayoutManager` was set, this will do nothing.
---
---@param output OutputHandle?
function layout.request_layout(output)
    if not layout.stream then
        return
    end

    local body = protobuf.encode(".pinnacle.layout.v0alpha1.LayoutRequest", {
        layout = {
            output_name = output and output.name,
        },
    })

    layout.stream:write_chunk(body, false)
end

---An object that manages layouts.
---@class LayoutManager
---@field layouts LayoutGenerator[]
---Get the active layout generator.
---@field get_active fun(self: self, args: LayoutArgs): LayoutGenerator

---@classmod
---A `LayoutManager` that keeps track of layouts per tag and provides
---methods to cycle between them.
---@class CyclingLayoutManager : LayoutManager
---@field tag_indices table<integer, integer>
local CyclingLayoutManager = {}

---@param args LayoutArgs
---
---@return LayoutGenerator
function CyclingLayoutManager:get_active(args)
    local first_tag = args.tags[1]

    -- Return a no-op generator if there are no active tags
    if not first_tag then
        ---@type LayoutGenerator
        return {
            layout = function(_, _)
                return {}
            end,
            gaps = 0,
        }
    end

    if not self.tag_indices[first_tag.id] then
        self.tag_indices[first_tag.id] = 1
    end

    return self.layouts[self.tag_indices[first_tag.id]]
end

---Cycle the layout for the given tag forward.
---
---@param tag TagHandle
function CyclingLayoutManager:cycle_layout_forward(tag)
    if not self.tag_indices[tag.id] then
        self.tag_indices[tag.id] = 1
    end

    self.tag_indices[tag.id] = self.tag_indices[tag.id] + 1

    if self.tag_indices[tag.id] > #self.layouts then
        self.tag_indices[tag.id] = 1
    end
end

---Cycle the layout for the given tag backward.
---
---@param tag TagHandle
function CyclingLayoutManager:cycle_layout_backward(tag)
    if not self.tag_indices[tag.id] then
        self.tag_indices[tag.id] = 1
    end

    self.tag_indices[tag.id] = self.tag_indices[tag.id] - 1

    if self.tag_indices[tag.id] < 1 then
        self.tag_indices[tag.id] = #self.layouts
    end
end

---Create a new cycling layout manager.
---
---@param layouts LayoutGenerator[]
---
---@return CyclingLayoutManager
---
---@see CyclingLayoutManager
function layout.new_cycling_manager(layouts)
    ---@type CyclingLayoutManager
    local self = {
        layouts = layouts,
        tag_indices = {},
    }

    setmetatable(self, { __index = CyclingLayoutManager })

    return self
end

return layout
