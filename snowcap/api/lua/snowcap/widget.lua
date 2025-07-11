-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@class snowcap.widget.Palette
---@field background snowcap.widget.Color
---@field text snowcap.widget.Color
---@field primary snowcap.widget.Color
---@field success snowcap.widget.Color
---@field warning snowcap.widget.Color
---@field danger snowcap.widget.Color

---@class snowcap.widget.Theme
---@field palette snowcap.widget.Palette?
---@field text_style snowcap.widget.text.Style?
---@field scrollable_style snowcap.widget.scrollable.Style?
---@field container_style snowcap.widget.container.Style?

---@class snowcap.widget.WidgetDef
---@field theme snowcap.widget.Theme?
---@field text snowcap.widget.Text?
---@field column snowcap.widget.Column?
---@field row snowcap.widget.Row?
---@field scrollable snowcap.widget.Scrollable?
---@field container snowcap.widget.Container?

---@class snowcap.widget.Border
---@field color snowcap.widget.Color?
---@field width number?
---@field radius snowcap.widget.Radius?

---@class snowcap.widget.Radius
---@field top_left number?
---@field top_right number?
---@field bottom_right number?
---@field bottom_left number?

---@class (exact) snowcap.widget.Text
---@field text string
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field halign snowcap.widget.Alignment?
---@field valign snowcap.widget.Alignment?
---@field style snowcap.widget.text.Style?

---@class snowcap.widget.text.Style
---@field color snowcap.widget.Color?
---@field pixels number?
---@field font snowcap.widget.Font?

---@class snowcap.widget.Column
---@field spacing number?
---@field padding snowcap.widget.Padding?
---@field item_alignment snowcap.widget.Alignment?
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field max_width number?
---@field clip boolean?
---@field children snowcap.widget.WidgetDef[]

---@class snowcap.widget.Row
---@field spacing number?
---@field padding snowcap.widget.Padding?
---@field item_alignment snowcap.widget.Alignment?
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field clip boolean?
---@field children snowcap.widget.WidgetDef[]

---@class snowcap.widget.Scrollable
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field direction snowcap.widget.scrollable.Direction?
---@field child snowcap.widget.WidgetDef
---@field style snowcap.widget.scrollable.Style?

---@class snowcap.widget.scrollable.Direction
---@field vertical snowcap.widget.scrollable.Scrollbar?
---@field horizontal snowcap.widget.scrollable.Scrollbar?

---@class snowcap.widget.scrollable.Scrollbar
---@field width_pixels number?
---@field height_pixels number?
---@field scroller_width_pixels number?
---@field anchor_to_end boolean?
---@field embed_spacing number?

---@class snowcap.widget.scrollable.Style
---@field container_style snowcap.widget.container.Style?
---@field vertical_rail snowcap.widget.scrollable.Rail?
---@field horizontal_rail snowcap.widget.scrollable.Rail?

---@class snowcap.widget.scrollable.Rail
---@field background_color snowcap.widget.Color?
---@field border snowcap.widget.Border?
---@field scroller_color snowcap.widget.Color?
---@field scroller_border snowcap.widget.Border?

---@class (exact) snowcap.widget.Container
---@field padding snowcap.widget.Padding?
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field max_width number?
---@field max_height number?
---@field halign snowcap.widget.Alignment?
---@field valign snowcap.widget.Alignment?
---@field clip boolean?
---@field child snowcap.widget.WidgetDef
---@field style snowcap.widget.container.Style?

---@class snowcap.widget.container.Style
---@field text_color snowcap.widget.Color?
---@field background_color snowcap.widget.Color?
---@field border snowcap.widget.Border?

---@class snowcap.widget.Length
---@field fill {}?
---@field fill_portion integer?
---@field shrink {}?
---@field fixed number?

local length = {
    ---@type snowcap.widget.Length
    Fill = { fill = {} },
    ---@type fun(portion: integer): snowcap.widget.Length
    FillPortion = function(portion)
        return { fill_portion = portion }
    end,
    ---@type snowcap.widget.Length
    Shrink = { shrink = {} },
    ---@type fun(size: number): snowcap.widget.Length
    Fixed = function(size)
        return { fixed = size }
    end,
}

---@enum snowcap.widget.Alignment
local alignment = {
    START = 1,
    CENTER = 2,
    END = 3,
}

---@class snowcap.widget.Color
---@field red number?
---@field green number?
---@field blue number?
---@field alpha number?

local color = {}

---@param r number
---@param g number
---@param b number
---@param a number?
---
---@return snowcap.widget.Color
function color.from_rgba(r, g, b, a)
    return {
        red = r,
        green = g,
        blue = b,
        alpha = a or 1.0,
    }
end

---@class snowcap.widget.Font
---@field family snowcap.Font.Family?
---@field weight snowcap.Font.Weight?
---@field stretch snowcap.Font.Stretch?
---@field style snowcap.Font.Style?

---@class snowcap.Font.Family
---@field name string?
---@field serif {}?
---@field sans_serif {}?
---@field cursive {}?
---@field fantasy {}?
---@field monospace {}?

local font = {
    family = {
        ---@type fun(name: string): snowcap.Font.Family
        Name = function(name)
            return { name = name }
        end,
        ---@type snowcap.Font.Family
        Serif = { serif = {} },
        ---@type snowcap.Font.Family
        SansSerif = { sans_serif = {} },
        ---@type snowcap.Font.Family
        Cursive = { cursive = {} },
        ---@type snowcap.Font.Family
        Fantasy = { fantasy = {} },
        ---@type snowcap.Font.Family
        Monospace = { monospace = {} },
    },

    ---@enum snowcap.Font.Weight
    weight = {
        THIN = 1,
        EXTRA_LIGHT = 2,
        LIGHT = 3,
        NORMAL = 4,
        MEDIUM = 5,
        SEMIBOLD = 6,
        BOLD = 7,
        EXTRA_BOLD = 8,
        BLACK = 9,
    },

    ---@enum snowcap.Font.Stretch
    stretch = {
        ULTRA_CONDENSED = 1,
        EXTRA_CONDENSED = 2,
        CONDENSED = 3,
        SEMI_CONDENSED = 4,
        NORMAL = 5,
        SEMI_EXPANDED = 6,
        EXPANDED = 7,
        EXTRA_EXPANDED = 8,
        ULTRA_EXPANDED = 9,
    },

    ---@enum snowcap.Font.Style
    style = {
        NORMAL = 1,
        ITALIC = 2,
        OBLIQUE = 3,
    },
}

---@class snowcap.widget.Padding
---@field top number?
---@field right number?
---@field bottom number?
---@field left number?

local widget = {
    length = length,
    alignment = alignment,
    color = color,
    font = font,
}

---@param def snowcap.widget.Text
---@return snowcap.widget.v1.Text
local function text_into_api(def)
    ---@type snowcap.widget.v1.Text
    return {
        text = def.text,
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        vertical_alignment = def.valign,
        horizontal_alignment = def.halign,
        style = def.style --[[@as snowcap.widget.v1.Text.Style]],
    }
end

---@param def snowcap.widget.Container
---@return snowcap.widget.v1.Container
local function container_into_api(def)
    ---@type snowcap.widget.v1.Container
    return {
        padding = def.padding --[[@as snowcap.widget.v1.Padding]],
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        max_width = def.max_width,
        max_height = def.max_height,
        vertical_alignment = def.valign,
        horizontal_alignment = def.halign,
        clip = def.clip,
        child = widget.widget_def_into_api(def.child),
        style = def.style --[[@as snowcap.widget.v1.Container.Style]],
    }
end

---@param def snowcap.widget.Column
---@return snowcap.widget.v1.Column
local function column_into_api(def)
    local children = {}
    for _, child in ipairs(def.children) do
        table.insert(children, widget.widget_def_into_api(child))
    end

    ---@type snowcap.widget.v1.Column
    return {
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        max_width = def.max_width,
        padding = def.padding --[[@as snowcap.widget.v1.Padding]],
        spacing = def.spacing,
        clip = def.clip,
        item_alignment = def.item_alignment,
        children = children,
    }
end

---@param def snowcap.widget.Row
---@return snowcap.widget.v1.Row
local function row_into_api(def)
    local children = {}
    for _, child in ipairs(def.children) do
        table.insert(children, widget.widget_def_into_api(child))
    end

    ---@type snowcap.widget.v1.Row
    return {
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        padding = def.padding --[[@as snowcap.widget.v1.Padding]],
        spacing = def.spacing,
        clip = def.clip,
        item_alignment = def.item_alignment,
        children = children,
    }
end

---@param def snowcap.widget.Scrollable
---@return snowcap.widget.v1.Scrollable
local function scrollable_into_api(def)
    ---@type snowcap.widget.v1.Scrollable
    return {
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        direction = def.direction --[[@as snowcap.widget.v1.Scrollable.Direction]],
        child = widget.widget_def_into_api(def.child),
    }
end

---@param def snowcap.widget.WidgetDef
---@return snowcap.widget.v1.WidgetDef
function widget.widget_def_into_api(def)
    if def.text then
        def.text = text_into_api(def.text)
    end
    if def.container then
        def.container = container_into_api(def.container)
    end
    if def.column then
        def.column = column_into_api(def.column)
    end
    if def.row then
        def.row = row_into_api(def.row)
    end
    if def.scrollable then
        def.scrollable = scrollable_into_api(def.scrollable)
    end

    return def --[[@as snowcap.widget.v1.WidgetDef]]
end

---@param text snowcap.widget.Text
---
---@return snowcap.widget.WidgetDef
function widget.text(text)
    return {
        text = text,
    }
end

---@param column snowcap.widget.Column
---
---@return snowcap.widget.WidgetDef
function widget.column(column)
    return {
        column = column,
    }
end

---@param row snowcap.widget.Row
---
---@return snowcap.widget.WidgetDef
function widget.row(row)
    return {
        row = row,
    }
end

---@param scrollable snowcap.widget.Scrollable
---
---@return snowcap.widget.WidgetDef
function widget.scrollable(scrollable)
    return {
        scrollable = scrollable,
    }
end

---@param container snowcap.widget.Container
---
---@return snowcap.widget.WidgetDef
function widget.container(container)
    return {
        container = container,
    }
end

return widget
