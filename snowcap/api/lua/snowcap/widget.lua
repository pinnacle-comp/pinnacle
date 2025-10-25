-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@class snowcap.widget.Program
---@field update fun(self: self, message: any)
---@field view fun(self: self): snowcap.widget.WidgetDef

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
---@field button snowcap.widget.Button?
---@field image snowcap.widget.Image?
---@field input_region snowcap.widget.InputRegion?
---@field mouse_area snowcap.widget.MouseArea?

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

---@class snowcap.widget.Container
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

---@class snowcap.widget.Button
---@field child snowcap.widget.WidgetDef
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field padding snowcap.widget.Padding?
---@field clip boolean?
---@field style snowcap.widget.button.Styles?
---@field on_press any?
---@field private widget_id integer?

---@class snowcap.widget.button.Styles
---@field active snowcap.widget.button.Style?
---@field hovered snowcap.widget.button.Style?
---@field pressed snowcap.widget.button.Style?
---@field disabled snowcap.widget.button.Style?

---@class snowcap.widget.button.Style
---@field text_color snowcap.widget.Color?
---@field background_color snowcap.widget.Color?
---@field border snowcap.widget.Border?

---@class snowcap.widget.Image
---@field handle snowcap.widget.image.Handle
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field expand boolean?
---@field content_fit snowcap.widget.image.ContentFit?
---@field nearest_neighbor boolean?
---@field rotation_degrees number?
---@field opacity number?
---@field scale number?

---@enum snowcap.widget.image.ContentFit
local content_fit = {
    CONTAIN = 1,
    COVER = 2,
    FILL = 3,
    NONE = 4,
    SCALE_DOWN = 5,
}

---@class snowcap.widget.image.Handle
---@field path string?
---@field bytes string?
---@field rgba { width: integer, height: integer, rgba: string }?

---@class snowcap.widget.InputRegion
---@field add boolean
---@field width snowcap.widget.Length?
---@field height snowcap.widget.Length?
---@field child snowcap.widget.WidgetDef

---Emits messages on mouse events.
---@class snowcap.widget.MouseArea
---@field child snowcap.widget.WidgetDef MouseArea content
---@field on_press any? Message to emit on a left button press.
---@field on_release any? Message to emit on a left button release.
---@field on_double_click any? Message to emit on a left button double click.
---@field on_right_press any? Message to emit on a right button press.
---@field on_right_release any? Message to emit on a right button release.
---@field on_middle_press any? Message to emit on a middle button press.
---@field on_middle_release any? Message to emit on a middle button release.
---@field on_scroll (fun(scroll_delta: snowcap.widget.mouse_area.ScrollEvent): any)? Message to emit when the scroll wheel is used.
---@field on_enter any? Message to emit when the mouse pointer enter the area.
---@field on_move (fun(point: snowcap.widget.mouse_area.MoveEvent): any)? Message to emit when the mouse move in the area.
---@field on_exit any? Message to emit when the mouse pointer exit the area.
---@field interaction snowcap.widget.mouse.Interaction? mouse.Interaction to use when hovering the area
---@field package widget_id integer?

---@class snowcap.widget.mouse_area.Callbacks
---@field on_press any? Message to emit on a left button press.
---@field on_release any? Message to emit on a left button release.
---@field on_double_click any? Message to emit on a left button double click.
---@field on_right_press any? Message to emit on a right button press.
---@field on_right_release any? Message to emit on a right button release.
---@field on_middle_press any? Message to emit on a middle button press.
---@field on_middle_release any? Message to emit on a middle button release.
---@field on_scroll (fun(scroll_delta: snowcap.widget.mouse_area.ScrollEvent): any)? Message to emit when the scroll wheel is used.
---@field on_enter any? Message to emit when the mouse pointer enter the area.
---@field on_move (fun(point: snowcap.widget.mouse_area.MoveEvent): any)? Message to emit when the mouse move in the area.
---@field on_exit any? Message to emit when the mouse pointer exit the area.

---@class snowcap.widget.mouse_area.Event
---@field press? {}
---@field release? {}
---@field double_click? {}
---@field right_press? {}
---@field right_release? {}
---@field middle_press? {}
---@field middle_release? {}
---@field scroll? snowcap.widget.mouse_area.ScrollEvent?
---@field enter? {}
---@field move? snowcap.widget.mouse_area.MoveEvent?
---@field exit? {}

---@enum snowcap.widget.mouse_area.event.Type
local mouse_area_event_type = {
    PRESS = "press",
    RELEASE = "release",
    DOUBLE_CLICK = "double_click",
    RIGHT_PRESS = "right_press",
    RIGHT_RELEASE = "right_release",
    MIDDLE_PRESS = "middle_press",
    MIDDLE_RELEASE = "middle_release",
    SCROLL = "scroll",
    ENTER = "enter",
    MOVE = "move",
    EXIT = "exit",
}

---@class snowcap.widget.mouse_area.ScrollEvent
---@field lines snowcap.widget.mouse_area.ScrollEvent.Lines?
---@field pixels snowcap.widget.mouse_area.ScrollEvent.Pixels?

---@class snowcap.widget.mouse_area.ScrollEvent.Lines
---@field x number?
---@field y number?

---@class snowcap.widget.mouse_area.ScrollEvent.Pixels
---@field x number?
---@field y number?

---@class snowcap.widget.mouse_area.MoveEvent
---@field x number?
---@field y number?

local mouse = {
    ---@enum snowcap.widget.mouse.Interaction
    interaction = {
        NONE = 0,
        IDLE = 1,
        POINTER = 2,
        GRAB = 3,
        TEXT = 4,
        CROSSHAIR = 5,
        GRABBING = 6,
        RESIZE_HORIZONTAL = 7,
        RESIZE_VERTICAL = 8,
        RESIZE_DIAGONAL_UP = 9,
        RESIZE_DIAGONAL_DOWN = 10,
        NOT_ALLOWED = 11,
        ZOOM_IN = 12,
        ZOOM_OUT = 13,
        CELL = 14,
        MOVE = 15,
        COPY = 16,
        HELP = 17,
    },
}

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

---@class snowcap.widget.Callback
---@field button fun(widget: snowcap.widget.WidgetDef)?
---@field mouse_area fun(widget: snowcap.widget.WidgetDef)?

local widget = {
    length = length,
    alignment = alignment,
    color = color,
    font = font,
    image = {
        content_fit = content_fit,
    },
    mouse = mouse,
}

local widget_id_counter = 0

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

---@param def snowcap.widget.Button
---@return snowcap.widget.v1.Button
local function button_into_api(def)
    ---@type snowcap.widget.v1.Button
    return {
        child = widget.widget_def_into_api(def.child),
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        padding = def.padding --[[@as snowcap.widget.v1.Padding]],
        clip = def.clip,
        style = def.style --[[@as snowcap.widget.v1.Button.Style]],
        widget_id = def.widget_id,
    }
end

---@param def snowcap.widget.Image
---@return snowcap.widget.v1.Image
local function image_into_api(def)
    ---@type snowcap.widget.v1.Image
    return {
        path = def.handle.path,
        bytes = def.handle.bytes,
        rgba = def.handle.rgba,
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
        expand = def.expand,
        content_fit = def.content_fit,
        nearest_neighbor = def.nearest_neighbor,
        rotation_degrees = def.rotation_degrees,
        opacity = def.opacity,
        scale = def.scale,
    }
end

---@param def snowcap.widget.InputRegion
---@return snowcap.widget.v1.InputRegion
local function input_region_into_api(def)
    ---@type snowcap.widget.v1.InputRegion
    return {
        add = def.add,
        child = widget.widget_def_into_api(def.child),
        width = def.width --[[@as snowcap.widget.v1.Length]],
        height = def.height --[[@as snowcap.widget.v1.Length]],
    }
end

---@param def snowcap.widget.MouseArea
---@return snowcap.widget.v1.MouseArea
local function mouse_area_into_api(def)
    ---@type snowcap.widget.v1.MouseArea
    return {
        child = widget.widget_def_into_api(def.child),
        on_press = def.on_press ~= nil,
        on_release = def.on_release ~= nil,
        on_double_click = def.on_double_click ~= nil,
        on_right_press = def.on_right_press ~= nil,
        on_right_release = def.on_right_release ~= nil,
        on_middle_press = def.on_middle_press ~= nil,
        on_middle_release = def.on_middle_release ~= nil,
        on_scroll = def.on_scroll ~= nil,
        on_enter = def.on_enter ~= nil,
        on_move = def.on_move ~= nil,
        on_exit = def.on_exit ~= nil,
        interaction = def.interaction, --[[@as snowcap.widget.v1.MouseArea.Interaction]]
        widget_id = def.widget_id,
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
    if def.button then
        def.button = button_into_api(def.button)
    end
    if def.image then
        def.image = image_into_api(def.image)
    end
    if def.input_region then
        def.input_region = input_region_into_api(def.input_region)
    end
    if def.mouse_area then
        def.mouse_area = mouse_area_into_api(def.mouse_area)
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

---@param button snowcap.widget.Button
---
---@return snowcap.widget.WidgetDef
function widget.button(button)
    if button.on_press then
        button.widget_id = widget_id_counter
        widget_id_counter = widget_id_counter + 1
    end

    ---@type snowcap.widget.WidgetDef
    return {
        button = button,
    }
end

---@param image snowcap.widget.Image
---
---@return snowcap.widget.WidgetDef
function widget.Image(image)
    ---@type snowcap.widget.WidgetDef
    return {
        image = image,
    }
end

---@param input_region snowcap.widget.InputRegion
---
---@return snowcap.widget.WidgetDef
function widget.input_region(input_region)
    ---@type snowcap.widget.WidgetDef
    return {
        input_region = input_region,
    }
end

---Create a new MouseArea widget.
---@param mouse_area snowcap.widget.MouseArea
---
---@return snowcap.widget.WidgetDef
function widget.mouse_area(mouse_area)
    local has_cb = false

    has_cb = has_cb or mouse_area.on_press ~= nil
    has_cb = has_cb or mouse_area.on_release ~= nil
    has_cb = has_cb or mouse_area.on_double_click ~= nil
    has_cb = has_cb or mouse_area.on_right_press ~= nil
    has_cb = has_cb or mouse_area.on_right_release ~= nil
    has_cb = has_cb or mouse_area.on_middle_press ~= nil
    has_cb = has_cb or mouse_area.on_middle_release ~= nil
    has_cb = has_cb or mouse_area.on_scroll ~= nil
    has_cb = has_cb or mouse_area.on_enter ~= nil
    has_cb = has_cb or mouse_area.on_move ~= nil
    has_cb = has_cb or mouse_area.on_exit ~= nil

    if has_cb then
        mouse_area.widget_id = widget_id_counter
        widget_id_counter = widget_id_counter + 1
    end

    ---@type snowcap.widget.WidgetDef
    return {
        mouse_area = mouse_area,
    }
end

---@private
---@lcat nodoc
---@param wgt snowcap.widget.WidgetDef
---@param callbacks table<integer, any>
---@param with_widget fun(callbacks: table<integer, any>, widget: snowcap.widget.WidgetDef)
function widget._traverse_widget_tree(wgt, callbacks, with_widget)
    with_widget(callbacks, wgt)
    if wgt.column then
        for _, w in ipairs(wgt.column.children or {}) do
            widget._traverse_widget_tree(w, callbacks, with_widget)
        end
    elseif wgt.row then
        for _, w in ipairs(wgt.row.children or {}) do
            widget._traverse_widget_tree(w, callbacks, with_widget)
        end
    elseif wgt.scrollable then
        widget._traverse_widget_tree(wgt.scrollable.child, callbacks, with_widget)
    elseif wgt.container then
        widget._traverse_widget_tree(wgt.container.child, callbacks, with_widget)
    elseif wgt.button then
        widget._traverse_widget_tree(wgt.button.child, callbacks, with_widget)
    elseif wgt.input_region then
        widget._traverse_widget_tree(wgt.input_region.child, callbacks, with_widget)
    elseif wgt.mouse_area then
        widget._traverse_widget_tree(wgt.mouse_area.child, callbacks, with_widget)
    end
end

---@package
---@lcat nodoc
---
---Collect `snowcap.widget.MouseArea` widget.
---@param mouse_area snowcap.widget.MouseArea
---@return snowcap.widget.mouse_area.Callbacks
local function collect_mouse_area_callbacks(mouse_area)
    return {
        on_press = mouse_area.on_press,
        on_release = mouse_area.on_release,
        on_double_click = mouse_area.on_double_click,
        on_right_press = mouse_area.on_right_press,
        on_right_release = mouse_area.on_right_release,
        on_middle_press = mouse_area.on_middle_press,
        on_middle_release = mouse_area.on_middle_release,
        on_scroll = mouse_area.on_scroll,
        on_enter = mouse_area.on_enter,
        on_move = mouse_area.on_move,
        on_exit = mouse_area.on_exit,
    }
end

---@private
---@lcat nodoc
---@param callbacks any[]
---@param wgt snowcap.widget.WidgetDef
function widget._collect_callbacks(callbacks, wgt)
    if wgt.button and wgt.button.on_press then
        callbacks[wgt.button.widget_id] = wgt.button.on_press
    end

    if wgt.mouse_area and wgt.mouse_area.widget_id then
        callbacks[wgt.mouse_area.widget_id] = collect_mouse_area_callbacks(wgt.mouse_area)
    end
end

---@private
---@lcat nodoc
---@param callbacks snowcap.widget.mouse_area.Callbacks
---@param event snowcap.widget.mouse_area.Event
---@return any?
function widget._mouse_area_process_event(callbacks, event)
    callbacks = callbacks or {}
    local translate = {
        [mouse_area_event_type.PRESS] = "on_press",
        [mouse_area_event_type.RELEASE] = "on_release",
        [mouse_area_event_type.DOUBLE_CLICK] = "on_double_click",
        [mouse_area_event_type.RIGHT_PRESS] = "on_right_press",
        [mouse_area_event_type.RIGHT_RELEASE] = "on_right_release",
        [mouse_area_event_type.MIDDLE_PRESS] = "on_middle_press",
        [mouse_area_event_type.MIDDLE_RELEASE] = "on_middle_release",
        [mouse_area_event_type.SCROLL] = "on_scroll",
        [mouse_area_event_type.ENTER] = "on_enter",
        [mouse_area_event_type.MOVE] = "on_move",
        [mouse_area_event_type.EXIT] = "on_exit",
    }

    local event_type = nil
    local cb = nil

    for k, v in pairs(translate) do
        if event[k] ~= nil then
            event_type = k
            cb = callbacks[v]

            break
        end
    end

    if cb == nil then
        return nil
    end

    local msg = nil

    if event_type == mouse_area_event_type.SCROLL then
        local ok, val = pcall(cb, event.scroll)

        if not ok then
            require("snowcap.log").error(val)
        else
            msg = val
        end
    elseif event_type == mouse_area_event_type.MOVE then
        local ok, val = pcall(cb, event.move)

        if not ok then
            require("snowcap.log").error(val)
        else
            msg = val
        end
    else
        msg = cb
    end

    return msg
end

return widget
