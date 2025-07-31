-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client

local widget = require("snowcap.widget")

---@class snowcap.decoration
local decoration = {}

local decoration_handle = {}

---@class snowcap.decoration.DecorationHandle
---@field id integer
---@field private update fun(msg: any)
local DecorationHandle = {}

---@param id integer
---@param update fun(msg: any)
---@return snowcap.decoration.DecorationHandle
function decoration_handle.new(id, update)
    ---@type snowcap.decoration.DecorationHandle
    local self = {
        id = id,
        update = update,
    }
    setmetatable(self, { __index = DecorationHandle })
    return self
end

---@class snowcap.decoration.Bounds
---@field left integer
---@field right integer
---@field top integer
---@field bottom integer

---@class snowcap.decoration.DecorationArgs
---@field program snowcap.widget.Program
---@field toplevel_identifier string
---@field bounds snowcap.decoration.Bounds
---@field extents snowcap.decoration.Bounds
---@field z_index integer

-- FIXME: duplicate from layer
---comment
---@param widget snowcap.widget.WidgetDef
---@param callbacks table<integer, any>
---@param with_widget fun(callbacks: table<integer, any>, widget: snowcap.widget.WidgetDef)
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

---@param args snowcap.decoration.DecorationArgs
---@return snowcap.decoration.DecorationHandle|nil handle A handle to the decoration surface, or nil if an error occurred.
function decoration.new_widget(args)
    ---@type table<integer, any>
    local callbacks = {}

    local widget_def = args.program:view()

    traverse_widget_tree(widget_def, callbacks, function(callbacks, widget)
        if widget.button and widget.button.on_press then
            callbacks[widget.button.widget_id] = widget.button.on_press
        end
    end)

    ---@type snowcap.decoration.v1.NewDecorationRequest
    local request = {
        widget_def = widget.widget_def_into_api(widget_def),
        bounds = args.bounds,
        extents = args.extents,
        z_index = args.z_index,
        foreign_toplevel_handle_identifier = args.toplevel_identifier,
    }

    local response, err = client:snowcap_decoration_v1_DecorationService_NewDecoration(request)

    if err then
        log.error(err)
        return nil
    end

    assert(response)

    if not response.decoration_id then
        log.error("no decoration_id received")
        return nil
    end

    local decoration_id = response.decoration_id or 0

    local err = client:snowcap_widget_v1_WidgetService_GetWidgetEvents({
        decoration_id = decoration_id,
    }, function(response)
        local widget_id = response.widget_id or 0
        if response.button then
            if callbacks[widget_id] then
                args.program:update(callbacks[widget_id])
                local widget_def = args.program:view()
                callbacks = {}

                traverse_widget_tree(widget_def, callbacks, function(callbacks, widget)
                    if widget.button and widget.button.on_press then
                        callbacks[widget.button.widget_id] = widget.button.on_press
                    end
                end)

                local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
                    decoration_id = decoration_id,
                    widget_def = widget.widget_def_into_api(widget_def),
                })
            end
        end
    end)

    return decoration_handle.new(decoration_id, function(msg)
        args.program:update(msg)
        local widget_def = args.program:view()
        callbacks = {}

        traverse_widget_tree(widget_def, callbacks, function(callbacks, widget)
            if widget.button and widget.button.on_press then
                callbacks[widget.button.widget_id] = widget.button.on_press
            end
        end)

        local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
            decoration_id = decoration_id,
            widget_def = widget.widget_def_into_api(widget_def),
        })
    end)
end

function DecorationHandle:close()
    local _, err = client:snowcap_decoration_v1_DecorationService_Close({ decoration_id = self.id })

    if err then
        log.error(err)
    end
end

---Sends a message to this decoration's `Program`.
---
---@param message any
function DecorationHandle:send_message(message)
    self.update(message)
end

return decoration
