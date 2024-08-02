-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---Create `Rectangle`s.
---@class RectangleModule
local rectangle = {}

---@classmod
---A rectangle with a position and size.
---@class Rectangle
---@field x number The x-position of the top-left corner
---@field y number The y-position of the top-left corner
---@field width number The width of the rectangle
---@field height number The height of the rectangle
local Rectangle = {}

---Split this rectangle along `axis` at `at`.
---
---If `thickness` is specified, the split will chop off a section of this
---rectangle from `at` to `at + thickness`.
---
---`at` is relative to the space this rectangle is in, not
---this rectangle's origin.
---
---@param axis "horizontal" | "vertical"
---@param at number
---@param thickness number?
---
---@return Rectangle rect1 The first rectangle.
---@return Rectangle|nil rect2 The second rectangle, if there is one.
function Rectangle:split_at(axis, at, thickness)
    ---@diagnostic disable-next-line: redefined-local
    local thickness = thickness or 0

    if axis == "horizontal" then
        -- Split is off to the top, at most chop off to `thickness`
        if at <= self.y then
            local diff = at - self.y + thickness
            if diff > 0 then
                self.y = self.y + diff
                self.height = self.height - diff
            end

            return self
        -- Split is to the bottom, then do nothing
        elseif at >= self.y + self.height then
            return self
        -- Split only chops bottom off
        elseif at + thickness >= self.y + self.height then
            local diff = (self.y + self.height) - at
            self.height = self.height - diff
            return self
        -- Do a split
        else
            local x = self.x
            local top_y = self.y
            local width = self.width
            local top_height = at - self.y

            local bot_y = at + thickness
            local bot_height = self.y + self.height - at - thickness

            local rect1 = rectangle.new(x, top_y, width, top_height)
            local rect2 = rectangle.new(x, bot_y, width, bot_height)

            return rect1, rect2
        end
    elseif axis == "vertical" then
        -- Split is off to the left, at most chop off to `thickness`
        if at <= self.x then
            local diff = at - self.x + thickness
            if diff > 0 then
                self.x = self.x + diff
                self.width = self.width - diff
            end

            return self
        -- Split is to the right, then do nothing
        elseif at >= self.x + self.width then
            return self
        -- Split only chops bottom off
        elseif at + thickness >= self.x + self.width then
            local diff = (self.x + self.width) - at
            self.width = self.width - diff
            return self
        -- Do a split
        else
            local left_x = self.x
            local y = self.y
            local left_width = at - self.x
            local height = self.height

            local right_x = at + thickness
            local right_width = self.x + self.width - at - thickness

            local rect1 = rectangle.new(left_x, y, left_width, height)
            local rect2 = rectangle.new(right_x, y, right_width, height)

            return rect1, rect2
        end
    end

    print("Invalid axis:", axis)
    os.exit(1)
end

---@return Rectangle
function rectangle.new(x, y, width, height)
    ---@type Rectangle
    local self = {
        x = x,
        y = y,
        width = width,
        height = height,
    }
    setmetatable(self, { __index = Rectangle })
    return self
end

---Parse a modeline string.
---
---@param modeline string
---
---@return Modeline|nil modeline A modeline if successful
---@return string|nil error An error message if any
local function parse_modeline(modeline)
    local args = modeline:gmatch("[^%s]+")

    local targs = {}

    for arg in args do
        table.insert(targs, arg)
    end

    local clock = tonumber(targs[1])
    local hdisplay = tonumber(targs[2])
    local hsync_start = tonumber(targs[3])
    local hsync_end = tonumber(targs[4])
    local htotal = tonumber(targs[5])
    local vdisplay = tonumber(targs[6])
    local vsync_start = tonumber(targs[7])
    local vsync_end = tonumber(targs[8])
    local vtotal = tonumber(targs[9])
    local hsync = targs[10]
    local vsync = targs[11]

    if
        not (
            clock
            and hdisplay
            and hsync_start
            and hsync_end
            and htotal
            and vdisplay
            and vsync_start
            and vsync_end
            and vtotal
            and hsync
            and vsync
        )
    then
        return nil, "one or more fields was missing"
    end

    local hsync_lower = string.lower(hsync)
    local vsync_lower = string.lower(vsync)

    if hsync_lower == "+hsync" then
        hsync = true
    elseif hsync_lower == "-hsync" then
        hsync = false
    else
        return nil, "invalid hsync: " .. hsync
    end

    if vsync_lower == "+vsync" then
        vsync = true
    elseif vsync_lower == "-vsync" then
        vsync = false
    else
        return nil, "invalid vsync: " .. vsync
    end

    ---@type Modeline
    return {
        clock = clock,
        hdisplay = hdisplay,
        hsync_start = hsync_start,
        hsync_end = hsync_end,
        htotal = htotal,
        vdisplay = vdisplay,
        vsync_start = vsync_start,
        vsync_end = vsync_end,
        vtotal = vtotal,
        hsync = hsync,
        vsync = vsync,
    }
end

---Utility functions.
---@class Util
local util = {
    rectangle = rectangle,
    output = {
        parse_modeline = parse_modeline,
    },
}

---Batch a set of requests that will be sent to the compositor all at once.
---
---Normally, all API calls are blocking. For example, calling `Window.get_all`
---then calling `WindowHandle.props` on each returned window handle will block
---after each `props` call waiting for the compositor to respond:
---
---```
---local handles = Window.get_all()
---
--- -- Collect all the props into this table
---local props = {}
---
--- -- This for loop will block after each call. If the compositor is running slowly
--- -- for whatever reason, this will take a long time to complete as it requests
--- -- properties sequentially.
---for i, handle in ipairs(handles) do
---    props[i] = handle:props()
---end
---```
---
---In order to mitigate this issue, you can batch up a set of API calls using this function.
---This will send all requests to the compositor at once without blocking, then wait for the compositor
---to respond.
---
---You must wrap each request in a function, otherwise they would just get
---evaluated at the callsite in a blocking manner.
---
---#### Example
---```lua
---local handles = window.get_all()
---
--- ---@type (fun(): WindowProperties)[]
---local requests = {}
---
--- -- Wrap each request to `props` in another function
---for i, handle in ipairs(handles) do
---    requests[i] = function()
---        return handle:props()
---    end
---end
---
--- -- Batch send these requests
---local props = require("pinnacle.util").batch(requests)
--- -- `props` now contains the `WindowProperties` of all the windows above
---```
---
---@generic T
---
---@param requests (fun(): T)[] The requests that you want to batch up, wrapped in a function.
---
---@return T[] responses The results of each request in the same order that they were in `requests`.
function util.batch(requests)
    if #requests == 0 then
        return {}
    end

    local loop = require("cqueues").new()

    local responses = {}

    for i, request in ipairs(requests) do
        loop:wrap(function()
            responses[i] = request()
        end)
    end

    loop:loop()

    return responses
end

-- Taken from the following stackoverflow answer:
-- https://stackoverflow.com/a/16077650
local function deep_copy_rec(obj, seen)
    seen = seen or {}
    if obj == nil then
        return nil
    end
    if seen[obj] then
        return seen[obj]
    end

    local no
    if type(obj) == "table" then
        no = {}
        seen[obj] = no

        for k, v in next, obj, nil do
            no[deep_copy_rec(k, seen)] = deep_copy_rec(v, seen)
        end
        setmetatable(no, deep_copy_rec(getmetatable(obj), seen))
    else -- number, string, boolean, etc
        no = obj
    end
    return no
end

---Creates a deep copy of an object.
---
---@generic T
---
---@param obj T The object to deep copy.
---
---@return T deep_copy A deep copy of `obj`
function util.deep_copy(obj)
    return deep_copy_rec(obj, nil)
end

---Creates a table with entries key->value and value->key for all given pairs.
---
---@generic T
---@param key_value_pairs T
---
---@return T bijective_table A table with pairs key->value and value->key
function util.bijective_table(key_value_pairs)
    local ret = {}

    for key, value in pairs(key_value_pairs) do
        ret[key] = value
        ret[value] = key
    end

    return ret
end

---Makes a table bijective by inserting `value = key` entries for every key-value pair.
---
---@param table table
function util.make_bijective(table)
    local temp = {}

    for k, v in pairs(table) do
        temp[v] = k
    end

    for k, v in pairs(temp) do
        table[k] = v
    end
end

return util
