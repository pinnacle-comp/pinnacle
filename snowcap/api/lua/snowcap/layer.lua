-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client
local layer_service = require("snowcap.grpc.defs").snowcap.layer.v0alpha1.LayerService
local input_service = require("snowcap.grpc.defs").snowcap.input.v0alpha1.InputService

local widget = require("snowcap.widget")

---@class Layer
local layer = {}

---@class LayerHandleModule
local layer_handle = {}

---@class LayerHandle
---@field id integer
local LayerHandle = {}

function layer_handle.new(id)
    ---@type LayerHandle
    local self = {
        id = id,
    }
    setmetatable(self, { __index = LayerHandle })
    return self
end

---@enum snowcap.Anchor
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

---@enum snowcap.KeyboardInteractivity
local keyboard_interactivity = {
    NONE = 1,
    ON_DEMAND = 2,
    EXCLUSIVE = 3,
}

---@enum snowcap.ZLayer
local zlayer = {
    BACKGROUND = 1,
    BOTTOM = 2,
    TOP = 3,
    OVERLAY = 4,
}

---@alias snowcap.ExclusiveZone
---| integer
---| "respect"
---| "ignore"

---@param zone snowcap.ExclusiveZone
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
---@field widget snowcap.WidgetDef
---@field width integer
---@field height integer
---@field anchor snowcap.Anchor?
---@field keyboard_interactivity snowcap.KeyboardInteractivity
---@field exclusive_zone snowcap.ExclusiveZone
---@field layer snowcap.ZLayer

---@param args LayerArgs
---@return LayerHandle|nil handle A handle to the layer surface, or nil if an error occurred.
function layer.new_widget(args)
    ---@type snowcap.layer.v0alpha1.NewLayerRequest
    local request = {
        layer = args.layer,
        exclusive_zone = exclusive_zone_to_api(args.exclusive_zone),
        width = args.width,
        height = args.height,
        anchor = args.anchor,
        keyboard_interactivity = args.keyboard_interactivity,
        widget_def = widget.widget_def_into_api(args.widget),
    }

    local response, err = client:unary_request(layer_service.NewLayer, request)

    if err then
        log:error(err)
        return nil
    end

    ---@cast response snowcap.layer.v0alpha1.NewLayerResponse

    if not response.layer_id then
        log:error("no layer_id received")
        return nil
    end

    return layer_handle.new(response.layer_id)
end

---@param on_press fun(mods: snowcap.input.Modifiers, key: snowcap.Key)
function LayerHandle:on_key_press(on_press)
    local err = client:server_streaming_request(
        input_service.KeyboardKey,
        { id = self.id },
        function(response)
            ---@cast response snowcap.input.v0alpha1.KeyboardKeyResponse

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
        log:error(err)
    end
end

function LayerHandle:close()
    local _, err = client:unary_request(layer_service.Close, { layer_id = self.id })

    if err then
        log:error(err)
    end
end

layer.anchor = anchor
layer.keyboard_interactivity = keyboard_interactivity
layer.zlayer = zlayer

return layer
