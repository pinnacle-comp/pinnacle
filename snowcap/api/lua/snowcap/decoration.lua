-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client

local widget = require("snowcap.widget")
local widget_signal = require("snowcap.widget.signal")

---@class snowcap.decoration
local decoration = {}

local decoration_handle = {}

---@class snowcap.decoration.DecorationHandle
---@field id integer
---@field private update fun(msg: any)
local DecorationHandle = {}

---@param id integer
---@param update fun(msg: any?)
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

---The bounds extending a rectangle.
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

---@param args snowcap.decoration.DecorationArgs
---@return snowcap.decoration.DecorationHandle|nil handle A handle to the decoration surface, or nil if an error occurred.
function decoration.new_widget(args)
    ---@type table<integer, any>
    local callbacks = {}

    local widget_def = args.program:view()
    if widget_def == nil then
        log.error("TopLevel program must return a view.")
        return nil
    end

    widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

    ---@type snowcap.decoration.v1.NewDecorationRequest
    local request = {
        widget_def = widget.widget_def_into_api(widget_def),
        bounds = args.bounds --[[@as snowcap.decoration.v1.Bounds]],
        extents = args.extents --[[@as snowcap.decoration.v1.Bounds]],
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

    ---@type fun(msg: any?)
    local update_on_msg = function(msg)
        if msg ~= nil then
            args.program:update(msg)
        end

        ---@diagnostic disable-next-line: redefined-local
        local _, err = client:snowcap_decoration_v1_DecorationService_RequestView({
            decoration_id = decoration_id,
        })

        if err then
            log.error(err)
        end
    end

    args.program:connect(widget_signal.redraw_needed, update_on_msg)
    args.program:connect(widget_signal.send_message, update_on_msg)

    local handle = decoration_handle.new(decoration_id, update_on_msg)

    args.program:created(widget.SurfaceHandle.from_decoration_handle(handle))

    local err = client:snowcap_widget_v1_WidgetService_GetWidgetEvents({
        decoration_id = decoration_id,
    }, function(response)
        for _, event in ipairs(response.widget_events) do
            ---@diagnostic disable-next-line:invisible
            local msg = widget._message_from_event(callbacks, event)

            if msg then
                local ok, update_err = pcall(function()
                    args.program:update(msg)
                end)
                if not ok then
                    log.error(update_err)
                end
            end
        end

        ---@diagnostic disable-next-line:redefined-local
        local widget_def = args.program:view()
        if widget_def == nil then
            error("TopLevel program must return a view")
        end
        callbacks = {}

        widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

        local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
            decoration_id = decoration_id,
            widget_def = widget.widget_def_into_api(widget_def),
        })
    end)

    return handle
end

---Convert a DecorationHandle into a Popup's ParentHandle
---@return snowcap.popup.ParentHandle
function DecorationHandle:as_parent()
    return require("snowcap.popup").parent.Decoration(self)
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

---Sends an `Operation` to this decoration.
---@param operation snowcap.widget.operation.Operation
function DecorationHandle:operate(operation)
    local _, err = client:snowcap_decoration_v1_DecorationService_OperateDecoration({
        decoration_id = self.id,
        operation = require("snowcap.widget.operation")._to_api(operation),
    })

    if err then
        log.error(err)
    end
end

---Sets the z-index at which this decoration will render.
---
---@param z_index integer
function DecorationHandle:set_z_index(z_index)
    local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
        decoration_id = self.id,
        z_index = z_index,
    })

    if err then
        log.error(err)
    end
end

---Sets this decoration's extents.
---
---The extents extend the drawable area of the decorated toplevel
---by the specified amounts in each direction.
---
---@param extents snowcap.decoration.Bounds
function DecorationHandle:set_extents(extents)
    local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
        decoration_id = self.id,
        extents = extents --[[@as snowcap.decoration.v1.Bounds]],
    })

    if err then
        log.error(err)
    end
end

---Sets this decoration's bounds.
---
---The bounds extend the geometry of the decorated toplevel
---by the specified amounts in each direction, causing parts or
---all of the decoration to be included.
---
---@param bounds snowcap.decoration.Bounds
function DecorationHandle:set_bounds(bounds)
    local _, err = client:snowcap_decoration_v1_DecorationService_UpdateDecoration({
        decoration_id = self.id,
        bounds = bounds --[[@as snowcap.decoration.v1.Bounds]],
    })

    if err then
        log.error(err)
    end
end

return decoration
