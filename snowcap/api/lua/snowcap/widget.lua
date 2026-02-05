-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---@class snowcap.widget.Program : snowcap.widget.base.Base
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
---@field text_input snowcap.widget.TextInput?

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
---@field wrapping snowcap.widget.Wrapping?

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
---@field background snowcap.widget.Background?
---@field border snowcap.widget.Border?
---@field scroller_background snowcap.widget.Background?
---@field scroller_border snowcap.widget.Border?

---@class snowcap.widget.Container
---@field id string?
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
---@field background snowcap.widget.Background?
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
---@field background snowcap.widget.Background?
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

---A field that can be filled with text.
---
---## Example
---Create a simple Layer with an automatically focused `TextInput`:
---
---```lua
---local Layer = require("snowcap.layer")
---local Operation = require("snowcap.widget.operation")
---local Widget = require("snowcap.widget")
---
---local TextInputProgram = {
---    INPUT_ID = "prompt",
---}
---
---function TextInputProgram:view()
---    return Widget.text_input({
---        placeholder = "placeholder:",
---        value = self.input_value or "",
---        id = self.INPUT_ID,
---        on_input = function(data) return { content_changed = data } end,
---        on_submit = { submit = {} },
---        width = Widget.length.Fixed(500.0)
---    })
---end
---
---function TextInputProgram:update(msg)
---    if msg.content_changed then
---        self.input_value = msg.content_changed
---    elseif msg.submit then
---        -- do something with the input.
---        self.input_value = ""
---    end
---end
---
---function TextInputProgram:show()
---    local handle = Layer.new_widget({
---        program = self,
---        anchor = nil,
---        keyboard_interactivity = Layer.keyboard_interactivity.EXCLUSIVE,
---        exclusive_zone = "respect",
---        layer = Layer.zlayer.OVERLAY,
---    })
---    if not handle then return end
---
---    -- Focus the input
---    handle:operate(Operation.focusable.Focus(self.INPUT_ID))
---    handle:on_key_press(function(_, key)
---        local Keys = require("snowcap.input.keys")
---        if key == Keys.Escape then handle:close() end
---        if key == Keys.i then
---            handle:operate(Operation.focusable.Focus(self.INPUT_ID))
---        end
---    end)
---end
---
---function text_input_program()
---    local instance = {
---        INPUT_ID = TextInputProgram.INPUT_ID,
---        input_value = "",
---    }
---    setmetatable(instance, { __index = TextInputProgram })
---    return instance
---end
---```
---@class snowcap.widget.TextInput
---Text to display when the field is empty.
---@field placeholder string
---TextInput content.
---@field value string
---Set the TextInput Id.
---
---This id can then be used to target this widget with `Operation`s.
---@field id string?
---Convert the `TextInput` into a secure password input.
---@field secure boolean?
---Sets the message that should be produced when some text is typed into the `TextInput`.
---
---If the field is not set, the `TextInput` will be disabled.
---@field on_input (fun(data:string): any)?
---Sets the message that should be produced when the `TextInput` is focused and the enter
---key is pressed.
---@field on_submit any?
---Sets the message that should be produced when some text is pasted into the `TextInput`.
---@field on_paste (fun(data:string): any)?
---Sets the `Font` of the `TextInput`.
---@field font snowcap.widget.Font?
---Sets the `Icon` of the `TextInput`.
---@field icon snowcap.widget.text_input.Icon?
---Sets the width of the `TextInput`.
---@field width snowcap.widget.Length?
---Sets the `Padding` of the `TextInput`.
---@field padding snowcap.widget.Padding?
---Sets the `LineHeight` of the `TextInput`.
---@field line_height snowcap.widget.LineHeight?
---Sets the horizontal `Alignment` of the `TextInput`.
---@field horizontal_alignment snowcap.widget.Alignment?
---Sets the style of the `TextInput`.
---@field style snowcap.widget.text_input.Styles?
---@field package widget_id integer?

---The `TextInput` callbacks.
---@class snowcap.widget.text_input.Callbacks
---Sets the message that should be produced when some text is typed into the `TextInput`.
---
---If the field is not set, the `TextInput` will be disabled.
---@field on_input (fun(data:string): any)?
---Sets the message that should be produced when the `TextInput` is focused and the enter
---key is pressed.
---@field on_submit any?
---Sets the message that should be produced when some text is pasted into the `TextInput`.
---@field on_paste (fun(data:string): any)?

---The content of the `Icon`.
---@class snowcap.widget.text_input.Icon
---The `Font` that will be used to display the `code_point`.
---@field font snowcap.widget.Font?
---The unicode code point that will be used as the icon.
---@field code_point integer?
---The font size of the content.
---@field pixels number?
---The spacing between the `Icon` and the text in a `TextInput`.
---@field spacing number?
---Whether the icon should be displayed on the right side of the `TextInput`.
---@field right_side boolean?

---Styles to apply to the `TextInput`.
---@class snowcap.widget.text_input.Styles
---Style to use when the `TextInput` is active.
---@field active snowcap.widget.text_input.Style?
---Style to use when the `TextInput` is hovered.
---@field hovered snowcap.widget.text_input.Style?
---Style to use when the `TextInput` is focused.
---@field focused snowcap.widget.text_input.Style?
---Style to use when the `TextInput` is focused & hovered.
---@field hover_focused snowcap.widget.text_input.Style?
---Style to use when the `TextInput` is disabled.
---@field disabled snowcap.widget.text_input.Style?

---Appearance of a `TextInput`.
---@class snowcap.widget.text_input.Style
---The `Background` style.
---@field background snowcap.widget.Background?
---The `Border` of the `TextInput`.
---@field border snowcap.widget.Border?
---The `Color` of the `Icon`.
---@field icon snowcap.widget.Color?
---The `Color` of the placeholder.
---@field placeholder snowcap.widget.Color?
---The `Color` of the content.
---@field value snowcap.widget.Color?
---The `Color` to use for the selection's highlight.
---@field selection snowcap.widget.Color?

---@class snowcap.widget.text_input.Event
---@field event_type snowcap.widget.text_input.event.Type?
---@field data string?

---@enum snowcap.widget.text_input.event.Type
local text_input_event_type = {
    INPUT = "input",
    SUBMIT = "submit",
    PASTE = "press",
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

---A fill which transitions colors progressively.
---@class snowcap.widget.Gradient
---A linear gradient that interpolates colors along a direction at a specific angle.
---@field linear snowcap.widget.gradient.Linear?

---A linear gradient.
---@class snowcap.widget.gradient.Linear
---How the `Gradient` is angled.
---@field radians number
---`ColorStop` to interpolates.
---
---ColorStops should be sorted by increasing offsets.
---ColorStops offsets should be in the range 0.0..=1.0.
---Up to 8 ColorStops are supported.
---@field stops snowcap.widget.gradient.ColorStop[]

---A point along a gradient vector where the specified `Color` is unmixed.
---@class snowcap.widget.gradient.ColorStop
---Offset along the gradient vector.
---@field offset number
---The color of the gradient at the specified `offset`.
---@field color snowcap.widget.Color

---The background of some element.
---@class snowcap.widget.Background
---A solid color.
---@field color snowcap.widget.Color?
---Interpolate between several colors.
---@field gradient snowcap.widget.Gradient?

---Builders for `Background`.
---@class snowcap.widget.background
---Builds a `Background` from a solid `Color`.
---@field Color fun(color: snowcap.widget.Color): snowcap.widget.Background
---Builds a `Background` from a `Linear` gradient.
---@field Linear fun(radians: number, stops: snowcap.widget.gradient.ColorStop[]): snowcap.widget.Background
local background = {
    ---@type fun(color: snowcap.widget.Color): snowcap.widget.Background
    Color = function(color)
        return { color = color }
    end,
    ---@type fun(radians: number, stops: snowcap.widget.gradient.ColorStop[]): snowcap.widget.Background
    Linear = function(radians, stops)
        ---@type snowcap.widget.gradient.Linear
        local linear = { radians = radians, stops = stops or {} }

        ---@type snowcap.widget.Gradient
        local gradient = { linear = linear }

        return { gradient = gradient }
    end,
}

---The height of a line of text in a paragraph.
---@class snowcap.widget.LineHeight
---A factor of the size of the text.
---@field relative number?
---An absolute height in logical pixels.
---@field absolute number?

---Builders for `LineHeight`
---@class snowcap.widget.line_height
---Builds a relative `LineHeight`.
---@field Relative fun(size: number): snowcap.widget.LineHeight
---Builds an absolute `LineHeight`.
---@field Absolute fun(size: number): snowcap.widget.LineHeight
local line_height = {
    ---@type fun(size: number): snowcap.widget.LineHeight
    Relative = function(size)
        return { relative = size }
    end,
    ---@type fun(size: number): snowcap.widget.LineHeight
    Absolute = function(size)
        return { absolute = size }
    end,
}

---@enum snowcap.widget.Wrapping
local wrapping = {
    NONE = 1,
    WORD = 2,
    GLYPH = 3,
    WORD_OR_GLYPH = 4,
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
---@field text_input fun(widget: snowcap.widget.WidgetDef)?

local widget = {
    length = length,
    alignment = alignment,
    background = background,
    color = color,
    font = font,
    image = {
        content_fit = content_fit,
    },
    line_height = line_height,
    wrapping = wrapping,
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
        wrapping = def.wrapping --[[@as snowcap.widget.v1.Wrapping]],
        style = def.style --[[@as snowcap.widget.v1.Text.Style]],
    }
end

---@param def snowcap.widget.Container
---@return snowcap.widget.v1.Container
local function container_into_api(def)
    ---@type snowcap.widget.v1.Container
    return {
        id = def.id,
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

---@param def snowcap.widget.TextInput
---@return snowcap.widget.v1.TextInput
local function text_input_into_api(def)
    ---@type snowcap.widget.v1.TextInput
    return {
        placeholder = def.placeholder,
        value = def.value,
        id = def.id,
        secure = def.secure,
        on_input = def.on_input ~= nil,
        on_submit = def.on_submit ~= nil,
        on_paste = def.on_paste ~= nil,
        font = def.font --[[@as snowcap.widget.v1.Font]],
        icon = def.icon --[[@as snowcap.widget.v1.TextInput.Icon]],
        width = def.width --[[@as snowcap.widget.v1.Length]],
        padding = def.padding --[[@as snowcap.widget.v1.Padding]],
        line_height = def.line_height --[[@as snowcap.widget.v1.LineHeight]],
        horizontal_alignment = def.horizontal_alignment --[[@as snowcap.widget.v1.Alignment]],
        style = def.style --[[@as snowcap.widget.v1.TextInput.Style]],
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
    if def.text_input then
        def.text_input = text_input_into_api(def.text_input)
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

---Create a new TextInput widget.
---@param text_input snowcap.widget.TextInput
---
---@return snowcap.widget.WidgetDef
function widget.text_input(text_input)
    local has_cb = false

    has_cb = has_cb or text_input.on_input ~= nil
    has_cb = has_cb or text_input.on_submit ~= nil
    has_cb = has_cb or text_input.on_paste ~= nil

    if has_cb then
        text_input.widget_id = widget_id_counter
        widget_id_counter = widget_id_counter + 1
    end

    ---@type snowcap.widget.WidgetDef
    return {
        text_input = text_input,
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

---@package
---@lcat nodoc
---
---Collect event callbacks from a `snowcap.widget.TextInput`
---@param text_input snowcap.widget.TextInput
---@return snowcap.widget.text_input.Callbacks
local function collect_text_input_callbacks(text_input)
    return {
        on_input = text_input.on_input,
        on_submit = text_input.on_submit,
        on_paste = text_input.on_paste,
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

    if wgt.text_input and wgt.text_input.widget_id then
        callbacks[wgt.text_input.widget_id] = collect_text_input_callbacks(wgt.text_input)
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

---@private
---@lcat nodoc
---@param callbacks snowcap.widget.text_input.Callbacks
---@param event snowcap.widget.text_input.Event
---@return any?
function widget._text_input_process_event(callbacks, event)
    callbacks = callbacks or {}

    local translate = {
        [text_input_event_type.INPUT] = "on_input",
        [text_input_event_type.SUBMIT] = "on_submit",
        [text_input_event_type.PASTE] = "on_paste",
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

    if event_type == text_input_event_type.SUBMIT then
        msg = cb
    else
        local ok, val = pcall(cb, event[event_type])

        if not ok then
            require("snowcap.log").error(val)
        else
            msg = val
        end
    end

    return msg
end

---@private
---@lcat nodoc
---@param callbacks any[]
---@param event snowcap.widget.v1.WidgetEvent
function widget._message_from_event(callbacks, event)
    local widget_id = event.widget_id or 0
    local msg = nil

    if event.button then
        msg = callbacks[widget_id]
    elseif event.mouse_area then
        if callbacks[widget_id] ~= nil then
            ---@diagnostic disable-next-line:param-type-mismatch
            msg = widget._mouse_area_process_event(callbacks[widget_id], event.mouse_area)
        end
    elseif event.text_input then
        if callbacks[widget_id] ~= nil then
            ---@diagnostic disable-next-line:param-type-mismatch
            msg = widget._text_input_process_event(callbacks[widget_id], event.text_input)
        end
    end

    return msg
end

widget.operation = require("snowcap.widget.operation")

---A handle to a surface.
---
---@class snowcap.widget.SurfaceHandle
---A handle to a layer surface.
---@field layer snowcap.layer.LayerHandle?
---A handle to a decoration surface.
---@field decoration snowcap.decoration.DecorationHandle?
---A handle to a popup surface.
---@field popup snowcap.popup.PopupHandle?
local SurfaceHandle = {}

---@type metatable
local SurfaceHandle_mt = {
    __index = SurfaceHandle,
    ---@param self snowcap.widget.SurfaceHandle
    __tostring = function(self)
        if self.layer then
            return ("SurfaceHandle{Layer#%d}"):format(self.layer.id)
        end

        if self.decoration then
            return ("SurfaceHandle{Decoration#%d}"):format(self.decoration.id)
        end

        if self.popup then
            return ("SurfaceHandle{Popup#%d}"):format(self.popup.id)
        end

        return "SurfaceHandle{Unknown}"
    end,
}

---Creates a SurfaceHandle from a LayerHandle.
---
---@param handle snowcap.layer.LayerHandle
function SurfaceHandle.from_layer_handle(handle)
    ---@type snowcap.widget.SurfaceHandle
    local self = {
        layer = handle,
    }
    return setmetatable(self, SurfaceHandle_mt)
end

---Creates a SurfaceHandle from a DecorationHandle.
---
---@param handle snowcap.decoration.DecorationHandle
function SurfaceHandle.from_decoration_handle(handle)
    ---@type snowcap.widget.SurfaceHandle
    local self = {
        decoration = handle,
    }
    return setmetatable(self, SurfaceHandle_mt)
end

---Creates a SurfaceHandle from a PopupHandle.
---
---@param handle snowcap.popup.PopupHandle
function SurfaceHandle.from_popup_handle(handle)
    ---@type snowcap.widget.SurfaceHandle
    local self = {
        popup = handle,
    }
    return setmetatable(self, SurfaceHandle_mt)
end

---Closes this surface.
function SurfaceHandle:close()
    if self.layer then
        self.layer:close()
    end

    if self.decoration then
        self.decoration:close()
    end

    if self.popup then
        self.popup:close()
    end
end

---Sends a message to this surface.
---
---@param message any
function SurfaceHandle:send_message(message)
    if self.layer then
        self.layer:send_message(message)
    end

    if self.decoration then
        self.decoration:send_message(message)
    end

    if self.popup then
        self.popup:send_message(message)
    end
end

---Sends a operation to this surface.
---
---@param operation snowcap.widget.operation.Operation
function SurfaceHandle:operate(operation)
    if self.layer then
        self.layer:operate(operation)
    end

    if self.decoration then
        self.decoration:operate(operation)
    end

    if self.popup then
        self.popup:operate(operation)
    end
end

---Converts this surface handle into a popup parent.
---
---@return snowcap.popup.ParentHandle
---
---@see snowcap.popup.ParentHandle
function SurfaceHandle:as_parent()
    if self.layer then
        return self.layer:as_parent()
    end

    if self.decoration then
        return self.decoration:as_parent()
    end

    if self.popup then
        return self.popup:as_parent()
    end

    error("SurfaceHandle was empty")
end

widget.SurfaceHandle = SurfaceHandle

return widget
