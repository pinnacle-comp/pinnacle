-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@class snowcap.WidgetDef
---@field text snowcap.Text?
---@field column snowcap.Column?
---@field row snowcap.Row?
---@field scrollable snowcap.Scrollable?
---@field container snowcap.Container?

---@class snowcap.Text
---@field text string
---@field size number?
---@field width snowcap.Length?
---@field height snowcap.Length?
---@field halign snowcap.Alignment?
---@field valign snowcap.Alignment?
---@field color snowcap.Color?
---@field font snowcap.Font?

---@class snowcap.Column
---@field spacing number?
---@field padding snowcap.Padding?
---@field item_alignment snowcap.Alignment?
---@field width snowcap.Length?
---@field height snowcap.Length?
---@field max_width number?
---@field clip boolean?
---@field children snowcap.WidgetDef[]

---@class snowcap.Row
---@field spacing number?
---@field padding snowcap.Padding?
---@field item_alignment snowcap.Alignment?
---@field width snowcap.Length?
---@field height snowcap.Length?
---@field clip boolean?
---@field children snowcap.WidgetDef[]

---@class snowcap.Scrollable
---@field width snowcap.Length?
---@field height snowcap.Length?
---@field direction snowcap.Scrollable.Direction?
---@field child snowcap.WidgetDef

---@class snowcap.Scrollable.Direction
---@field vertical snowcap.Scrollable.Properties?
---@field horizontal snowcap.Scrollable.Properties?

---@class snowcap.Scrollable.Properties
---@field width number?
---@field height number?
---@field scroller_width number?
---@field alignment snowcap.Scrollable.Alignment?

---@class snowcap.Container
---@field padding snowcap.Padding?
---@field width snowcap.Length?
---@field height snowcap.Length?
---@field max_width number?
---@field max_height number?
---@field halign snowcap.Alignment?
---@field valign snowcap.Alignment?
---@field clip boolean?
---@field child snowcap.WidgetDef
---@field text_color snowcap.Color?
---@field background_color snowcap.Color?
---@field border_radius number?
---@field border_thickness number?
---@field border_color snowcap.Color?

local scrollable = {
    ---@enum snowcap.Scrollable.Alignment
    alignment = {
        START = 1,
        END = 2,
    },
}

---@class snowcap.Length
---@field fill {}?
---@field fill_portion integer?
---@field shrink {}?
---@field fixed number?

local length = {
    ---@type snowcap.Length
    Fill = { fill = {} },
    ---@type fun(portion: integer): snowcap.Length
    FillPortion = function(portion)
        return { fill_portion = portion }
    end,
    ---@type snowcap.Length
    Shrink = { shrink = {} },
    ---@type fun(size: number): snowcap.Length
    Fixed = function(size)
        return { fixed = size }
    end,
}

---@enum snowcap.Alignment
local alignment = {
    START = 1,
    CENTER = 2,
    END = 3,
}

---@class snowcap.Color
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
---@return snowcap.Color
function color.from_rgba(r, g, b, a)
    return {
        red = r,
        green = g,
        blue = b,
        alpha = a or 1.0,
    }
end

---@class snowcap.Font
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

---@class snowcap.Padding
---@field top number?
---@field right number?
---@field bottom number?
---@field left number?

local widget = {
    scrollable = scrollable,
    length = length,
    alignment = alignment,
    color = color,
    font = font,
}

---@param def snowcap.Text
---@return snowcap.widget.v0alpha1.Text
local function text_into_api(def)
    ---@type snowcap.widget.v0alpha1.Text
    return {
        text = def.text,
        pixels = def.size,
        width = def.width --[[@as snowcap.widget.v0alpha1.Length]],
        height = def.height --[[@as snowcap.widget.v0alpha1.Length]],
        vertical_alignment = def.valign,
        horizontal_alignment = def.halign,
        color = def.color --[[@as snowcap.widget.v0alpha1.Color]],
        font = def.font --[[@as snowcap.widget.v0alpha1.Font]],
    }
end

---@param def snowcap.Container
---@return snowcap.widget.v0alpha1.Container
local function container_into_api(def)
    ---@type snowcap.widget.v0alpha1.Container
    return {
        padding = def.padding --[[@as snowcap.widget.v0alpha1.Padding]],
        width = def.width --[[@as snowcap.widget.v0alpha1.Length]],
        height = def.height --[[@as snowcap.widget.v0alpha1.Length]],
        max_width = def.max_width,
        max_height = def.max_height,
        vertical_alignment = def.valign,
        horizontal_alignment = def.halign,
        clip = def.clip,
        child = widget.widget_def_into_api(def.child),
        text_color = def.text_color --[[@as snowcap.widget.v0alpha1.Color]],
        background_color = def.background_color --[[@as snowcap.widget.v0alpha1.Color]],
        border_radius = def.border_radius,
        border_thickness = def.border_thickness,
        border_color = def.border_color --[[@as snowcap.widget.v0alpha1.Color]],
    }
end

---@param def snowcap.Column
---@return snowcap.widget.v0alpha1.Column
local function column_into_api(def)
    local children = {}
    for _, child in ipairs(def.children) do
        table.insert(children, widget.widget_def_into_api(child))
    end

    ---@type snowcap.widget.v0alpha1.Column
    return {
        width = def.width --[[@as snowcap.widget.v0alpha1.Length]],
        height = def.height --[[@as snowcap.widget.v0alpha1.Length]],
        max_width = def.max_width,
        padding = def.padding --[[@as snowcap.widget.v0alpha1.Padding]],
        spacing = def.spacing,
        clip = def.clip,
        item_alignment = def.item_alignment,
        children = children,
    }
end

---@param def snowcap.Row
---@return snowcap.widget.v0alpha1.Row
local function row_into_api(def)
    local children = {}
    for _, child in ipairs(def.children) do
        table.insert(children, widget.widget_def_into_api(child))
    end

    ---@type snowcap.widget.v0alpha1.Row
    return {
        width = def.width --[[@as snowcap.widget.v0alpha1.Length]],
        height = def.height --[[@as snowcap.widget.v0alpha1.Length]],
        padding = def.padding --[[@as snowcap.widget.v0alpha1.Padding]],
        spacing = def.spacing,
        clip = def.clip,
        item_alignment = def.item_alignment,
        children = children,
    }
end

---@param def snowcap.Scrollable
---@return snowcap.widget.v0alpha1.Scrollable
local function scrollable_into_api(def)
    ---@type snowcap.widget.v0alpha1.Scrollable
    return {
        width = def.width --[[@as snowcap.widget.v0alpha1.Length]],
        height = def.height --[[@as snowcap.widget.v0alpha1.Length]],
        direction = def.direction --[[@as snowcap.widget.v0alpha1.ScrollableDirection]],
        child = widget.widget_def_into_api(def.child),
    }
end

---@param def snowcap.WidgetDef
---@return snowcap.widget.v0alpha1.WidgetDef
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

    return def --[[@as snowcap.widget.v0alpha1.WidgetDef]]
end

---@param text snowcap.Text
---
---@return snowcap.WidgetDef
function widget.text(text)
    return {
        text = text,
    }
end

---@param column snowcap.Column
---
---@return snowcap.WidgetDef
function widget.column(column)
    return {
        column = column,
    }
end

---@param row snowcap.Row
---
---@return snowcap.WidgetDef
function widget.row(row)
    return {
        row = row,
    }
end

---@param scrollable snowcap.Scrollable
---
---@return snowcap.WidgetDef
function widget.scrollable(scrollable)
    return {
        scrollable = scrollable,
    }
end

---@param container snowcap.Container
---
---@return snowcap.WidgetDef
function widget.container(container)
    return {
        container = container,
    }
end

return widget
