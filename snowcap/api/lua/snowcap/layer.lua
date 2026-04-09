-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client

local widget = require("snowcap.widget")
local widget_signal = require("snowcap.widget.signal")

---@class snowcap.layer
local layer = {}

local layer_handle = {}

---@class snowcap.layer.LayerHandle
---@field id integer
---@field private _update fun(msg:any)
---@field private _operate fun(oper: snowcap.widget.operation.Operation)
local LayerHandle = {}

---Convert a LayerHandle into a Popup's ParentHandle
---@return snowcap.popup.ParentHandle
function LayerHandle:as_parent()
    return require("snowcap.popup").parent.Layer(self)
end

---@param id integer
---@param update fun(msg: any?)
---@param operate fun(oper: snowcap.widget.operation.Operation)
---@return snowcap.layer.LayerHandle
function layer_handle.new(id, update, operate)
    ---@type snowcap.layer.LayerHandle
    local self = {
        id = id,
        _update = update,
        _operate = operate,
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

---@package
---@enum snowcap.layer.FocusEvent
local focus_event = {
    GAINED = 1,
    LOST = 2,
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

---@class snowcap.layer.LayerArgs
---@field program snowcap.widget.Program
---@field anchor snowcap.layer.Anchor?
---@field keyboard_interactivity snowcap.layer.KeyboardInteractivity
---@field exclusive_zone snowcap.layer.ExclusiveZone
---@field layer snowcap.layer.ZLayer

---@param args snowcap.layer.LayerArgs
---@return snowcap.layer.LayerHandle|nil handle A handle to the layer surface, or nil if an error occurred.
function layer.new_widget(args)
    ---@type table<integer, any>
    local callbacks = {}

    local widget_def = args.program:view() or widget.row({ children = {} })

    widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

    ---@type snowcap.layer.v1.NewLayerRequest
    local request = {
        layer = args.layer,
        exclusive_zone = exclusive_zone_to_api(args.exclusive_zone),
        anchor = args.anchor,
        keyboard_interactivity = args.keyboard_interactivity,
        widget_def = widget.widget_def_into_api(widget_def),
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

    local layer_id = response.layer_id or 0

    local sender, receiver = require("snowcap.util.channel").spsc()

    ---@type fun(msg: any?)
    local update_on_msg = function(msg)
        if msg ~= nil then
            sender:send({
                message = msg
            })
        else
            sender:send({
                redraw = {}
            })
        end
    end

    ---@type fun(oper: snowcap.widget.operation.Operation)
    local operate_surface = function(oper)
        sender:send({
            operation = oper
        })
    end

    local handle = layer_handle.new(layer_id, update_on_msg, operate_surface)

    ---@type fun(): snowcap.signal.HandlerPolicy
    local close_surface = function()
        handle:close()

        return require("snowcap.signal").HandlerPolicy.Discard
    end

    args.program:connect(widget_signal.redraw_needed, update_on_msg)
    args.program:connect(widget_signal.send_message, update_on_msg)
    args.program:connect(widget_signal.operation, operate_surface)
    args.program:connect(widget_signal.request_close, close_surface)

    args.program:event({
        created = widget.SurfaceHandle.from_layer_handle(handle),
    })

    err = client:snowcap_layer_v1_LayerService_GetLayerEvents({
        layer_id = layer_id,
    }, function(response) ---@diagnostic disable-line:redefined-local
        sender:send({
            layer_events = response.layer_events or {}
        })
    end)

    err = client:snowcap_widget_v1_WidgetService_GetWidgetEvents({
        layer_id = layer_id,
    }, function(response) ---@diagnostic disable-line:redefined-local
        sender:send({
            widget_events = response.widget_events or {}
        })
    end)

    client.loop:wrap(function()
        local pending_operations = {}

        local msg = receiver:recv()
        while msg do
            local update_view = false;

            if msg.widget_events then
                for _, event in ipairs(msg.widget_events) do
                    ---@diagnostic disable-next-line:invisible
                    local message = widget._message_from_event(callbacks, event)

                    if message then
                        local ok, update_err = pcall(function()
                            args.program:update(message)
                        end)
                        if not ok then
                            log.error(update_err)
                        end
                    end
                end
                update_view = true
            elseif msg.message then
                args.program:update(msg.message)
            elseif msg.operation then
                table.insert(pending_operations, msg.operation)
            elseif msg.layer_events then
                for _, layer_event in ipairs(msg.layer_events) do
                    if layer_event.closing ~= nil then
                        goto main_loop_break
                    end

                    local focus = layer_event.focus --[[@as snowcap.layer.FocusEvent]]
                    ---@type snowcap.widget.SurfaceEvent?
                    local event = nil

                    if focus == focus_event.GAINED then
                        event = {
                            focus_gained = {},
                        }
                    elseif focus == focus_event.LOST then
                        event = {
                            focus_lost = {},
                        }
                    end

                    if event then
                        args.program:event(event)
                    end
                end
            end

            if not update_view then
                ---@diagnostic disable-next-line: redefined-local
                local _, err = client:snowcap_layer_v1_LayerService_RequestView({
                    layer_id = layer_id,
                })

                if err then
                    log.error(err)
                end
            else
                ---@diagnostic disable-next-line:redefined-local
                local widget_def = args.program:view() or widget.row({ children = {} })
                callbacks = {}

                widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

                ---@diagnostic disable-next-line:redefined-local
                local _, err = client:snowcap_layer_v1_LayerService_UpdateLayer({
                    layer_id = layer_id,
                    widget_def = widget.widget_def_into_api(widget_def),
                })

                if err then
                    log.error(err)
                end

                for _, oper in pairs(pending_operations) do
                    ---@diagnostic disable-next-line:redefined-local
                    local _, err = client:snowcap_layer_v1_LayerService_OperateLayer({
                        layer_id = layer_id,
                        operation = require("snowcap.widget.operation")._to_api(oper), ---@diagnostic disable-line: invisible
                    })

                    if err then
                        log.error(err)
                    end
                end
                pending_operations = {}
            end

            msg = receiver:recv()
        end
        ::main_loop_break::

        args.program:event({
            closing = {},
        })
    end)

    return handle
end

---Do something when a key event is received.
---@param on_event fun(handle: snowcap.layer.LayerHandle, event: snowcap.input.KeyEvent)
function LayerHandle:on_key_event(on_event)
    local err = client:snowcap_input_v1_InputService_KeyboardKey(
        { layer_id = self.id },
        function(response)
            ---@cast response snowcap.input.v1.KeyboardKeyResponse

            local mods = response.modifiers or {}
            mods.shift = mods.shift or false
            mods.ctrl = mods.ctrl or false
            mods.alt = mods.alt or false
            mods.super = mods.super or false

            ---@cast mods snowcap.input.Modifiers

            ---@type snowcap.input.KeyEvent
            local event = {
                key = response.key or 0,
                mods = mods,
                pressed = response.pressed,
                captured = response.captured,
                text = response.text,
            }

            on_event(self, event)
        end
    )

    if err then
        log.error(err)
    end
end

---@param on_press fun(mods: snowcap.input.Modifiers, key: snowcap.Key)
function LayerHandle:on_key_press(on_press)
    self:on_key_event(function(_, event)
        if not event.pressed or event.captured then
            return
        end

        on_press(event.mods, event.key)
    end)
end

---@class snowcap.layer.LayerUpdateArgs
---@field anchor? snowcap.layer.Anchor
---@field keyboard_interactivity? snowcap.layer.KeyboardInteractivity
---@field exclusive_zone? snowcap.layer.ExclusiveZone
---@field layer? snowcap.layer.ZLayer

---Update this layer's attributes.
---@param args snowcap.layer.LayerUpdateArgs
---@return boolean True if the operation succeed.
function LayerHandle:update(args)
    local exclusive_zone = args.exclusive_zone and exclusive_zone_to_api(args.exclusive_zone) or nil

    local _, err = client:snowcap_layer_v1_LayerService_UpdateLayer({
        layer_id = self.id,
        anchor = args.anchor,
        keyboard_interactivity = args.keyboard_interactivity,
        exclusive_zone = exclusive_zone,
        layer = args.layer,
    })

    if err then
        log.error(err)
    end

    return err == nil
end

function LayerHandle:close()
    local _, err = client:snowcap_layer_v1_LayerService_Close({ layer_id = self.id })

    if err then
        log.error(err)
    end
end

function LayerHandle:send_message(message)
    self._update(message)
end

---Sends an `Operation` to this layer.
---@param operation snowcap.widget.operation.Operation
function LayerHandle:operate(operation)
    self._operate(operation)
end

layer.anchor = anchor
layer.keyboard_interactivity = keyboard_interactivity
layer.zlayer = zlayer

return layer
