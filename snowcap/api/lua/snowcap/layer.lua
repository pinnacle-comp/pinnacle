-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client

local widget = require("snowcap.widget")

---@class snowcap.layer
local layer = {}

local layer_handle = {}

---@class snowcap.layer.LayerHandle
---@field id integer
local LayerHandle = {}

function layer_handle.new(id)
    ---@type snowcap.layer.LayerHandle
    local self = {
        id = id,
    }
    setmetatable(self, { __index = LayerHandle })
    return self
end

---@enum snowcap.layer.Anchor
local anchor = {
    TOP = 1,
    BOTTOM = 2,
    LEFT = 3,
    RIGHT = 4,
    TOP_LEFT = 5,
    TOP_RIGHT = 6,
    BOTTOM_LEFT = 7,
    BOTTOM_RIGHT = 8,
}

---@enum snowcap.layer.KeyboardInteractivity
local keyboard_interactivity = {
    NONE = 1,
    ON_DEMAND = 2,
    EXCLUSIVE = 3,
}

---@enum snowcap.layer.ZLayer
local zlayer = {
    BACKGROUND = 1,
    BOTTOM = 2,
    TOP = 3,
    OVERLAY = 4,
}

---@alias snowcap.layer.ExclusiveZone
---| integer
---| "respect"
---| "ignore"

---@param zone snowcap.layer.ExclusiveZone
---@return integer
local function exclusive_zone_to_api(zone)
    if type(zone) == "number" then
        return zone
    end

    if zone == "respect" then
        return 0
    end

    return -1
end

---@class LayerArgs
---@field widget snowcap.widget.WidgetDef
---@field width integer
---@field height integer
---@field anchor snowcap.layer.Anchor?
---@field keyboard_interactivity snowcap.layer.KeyboardInteractivity
---@field exclusive_zone snowcap.layer.ExclusiveZone
---@field layer snowcap.layer.ZLayer

---comment
---@param widget snowcap.widget.WidgetDef
---@param callbacks table<integer, snowcap.widget.Callback>
---@param with_widget fun(callbacks: table<integer, snowcap.widget.Callback>, widget: snowcap.widget.WidgetDef)
local function traverse_widget_tree(widget, callbacks, with_widget)
    with_widget(callbacks, widget)
    if widget.column then
        for _, w in ipairs(widget.column.children or {}) do
            traverse_widget_tree(w, callbacks, with_widget)
        end
    elseif widget.row then
        for _, w in ipairs(widget.row.children or {}) do
            traverse_widget_tree(w, callbacks, with_widget)
        end
    elseif widget.scrollable then
        traverse_widget_tree(widget.scrollable.child, callbacks, with_widget)
    elseif widget.container then
        traverse_widget_tree(widget.container.child, callbacks, with_widget)
    elseif widget.button then
        traverse_widget_tree(widget.button.child, callbacks, with_widget)
    end
end

---@param args LayerArgs
---@return snowcap.layer.LayerHandle|nil handle A handle to the layer surface, or nil if an error occurred.
function layer.new_widget(args)
    ---@type table<integer, snowcap.widget.Callback>
    local callbacks = {}
    traverse_widget_tree(args.widget, callbacks, function(callbacks, widget)
        if widget.button and widget.button.on_press then
            callbacks[widget.button.widget_id] = {
                button = widget.button.on_press,
            }
        end
    end)

    ---@type snowcap.layer.v1.NewLayerRequest
    local request = {
        layer = args.layer,
        exclusive_zone = exclusive_zone_to_api(args.exclusive_zone),
        width = args.width,
        height = args.height,
        anchor = args.anchor,
        keyboard_interactivity = args.keyboard_interactivity,
        widget_def = widget.widget_def_into_api(args.widget),
    }

    local response, err = client:snowcap_layer_v1_LayerService_NewLayer(request)

    if err then
        log.error(err)
        return nil
    end

    assert(response)

    if not response.layer_id then
        log.error("no layer_id received")
        return nil
    end

    local layer_id = response.layer_id

    local err = client:snowcap_widget_v1_WidgetService_GetWidgetEvents({
        layer_id = layer_id,
    }, function(response)
        local widget_id = response.widget_id or 0
        if response.button then
            if callbacks[widget_id] and callbacks[widget_id].button then
                callbacks[widget_id].button(args.widget)
                -- TODO: update layer after this cb
            end
        end
    end)

    return layer_handle.new(layer_id)
end

---@param on_press fun(mods: snowcap.input.Modifiers, key: snowcap.Key)
function LayerHandle:on_key_press(on_press)
    local err = client:snowcap_input_v1_InputService_KeyboardKey(
        { id = self.id },
        function(response)
            ---@cast response snowcap.input.v1.KeyboardKeyResponse

            if not response.pressed then
                return
            end

            local mods = response.modifiers or {}
            mods.shift = mods.shift or false
            mods.ctrl = mods.ctrl or false
            mods.alt = mods.alt or false
            mods.super = mods.super or false

            ---@cast mods snowcap.input.Modifiers

            on_press(mods, response.key or 0)
        end
    )

    if err then
        log.error(err)
    end
end

function LayerHandle:close()
    local _, err = client:snowcap_layer_v1_LayerService_Close({ layer_id = self.id })

    if err then
        log.error(err)
    end
end

layer.anchor = anchor
layer.keyboard_interactivity = keyboard_interactivity
layer.zlayer = zlayer

return layer
