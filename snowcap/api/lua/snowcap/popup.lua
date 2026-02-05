-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("snowcap.log")
local client = require("snowcap.grpc.client").client

local widget = require("snowcap.widget")
local widget_signal = require("snowcap.widget.signal")

---Support for popup surface widgets using `xdg-shell::xdg_popup`
---@class snowcap.popup
local popup = {}

local popup_handle = {}

---A handle to a popup's parent surface.
---@class snowcap.popup.ParentHandle
---@field layer? integer Popup's parent surface is a layer.
---@field decoration? integer Popup's parent surface is a decoration.
---@field popup? integer Popup's parent surface is another popup.
local ParentHandle = {}

function ParentHandle:__tostring()
    if self.layer ~= nil then
        return ("ParentHandle{Layer#%d}"):format(self.layer)
    elseif self.decoration ~= nil then
        return ("ParentHandle{Decoration#%d}"):format(self.decoration)
    elseif self.popup ~= nil then
        return ("ParentHandle{Popup#%d}"):format(self.popup)
    else
        return "ParentHandle{empty}"
    end
end

---Build a handle to a popup's parent surface.
---@enum snowcap.popup.parent
local parent = {
    ---Build a ParentHandle from a LayerHandle.
    ---
    ---@param handle snowcap.layer.LayerHandle
    ---@return snowcap.popup.ParentHandle
    Layer = function(handle)
        return setmetatable({ layer = handle.id }, ParentHandle)
    end,

    ---Build a ParentHandle from a DecorationHandle.
    ---
    ---@param handle snowcap.decoration.DecorationHandle
    ---@return snowcap.popup.ParentHandle
    Decoration = function(handle)
        return setmetatable({ decoration = handle.id }, ParentHandle)
    end,

    ---Build a ParentHandle from a PopupHandle.
    ---
    ---@param handle snowcap.popup.PopupHandle
    ---@return snowcap.popup.ParentHandle
    Popup = function(handle)
        return setmetatable({ popup = handle.id }, ParentHandle)
    end,
}
popup.parent = parent

---A handle to a popup surface.
---@class snowcap.popup.PopupHandle
---@field id integer Popup's id.
---@field private _update fun(msg:any)
local PopupHandle = {}

---Convert a PopupHandle into a Popup's ParentHandle
---@return snowcap.popup.ParentHandle
function PopupHandle:as_parent()
    return parent.Popup(self)
end

---Create a new popup handle.
---@lcat nodoc
---@package
---@param id integer
---@param update fun(msg: any)
---@return snowcap.popup.PopupHandle
function popup_handle.new(id, update)
    ---@type snowcap.popup.PopupHandle
    local self = {
        id = id,
        _update = update,
    }
    setmetatable(self, { __index = PopupHandle })
    return self
end

---Anchoring rectangle.
---@class snowcap.popup.Rectangle
---@field x number
---@field y number
---@field width number
---@field height number

---Position the Popup will be placed at.
---
---This is an implementation detail. Use [`snowcap.popup.position`] instead.
---@class snowcap.popup.Position
---@field package at_cursor? {} Position the popup at the cursor.
---@field package absolute? snowcap.popup.Rectangle Position the popup on an arbitrary Rectangle boundaries.
---@field package at_widget? string Position the popup on a Widget boundaries.

---Position the Popup will be placed at.
---@enum snowcap.popup.position
local position = {
    ---Position the popup at the cursor.
    ---@type snowcap.popup.Position
    AtCursor = { at_cursor = {} },
    ---Position the anchor at an arbitrary point.
    ---@type fun(x: number, y: number): snowcap.popup.Position
    Point = function(x, y)
        return {
            absolute = {
                x = x,
                y = y,
                width = 1,
                height = 1,
            },
        }
    end,
    ---Position the anchor on a Rectangle boundaries.
    ---@type fun(x: number, y: number, width: number, heigh: number): snowcap.popup.Position
    Rectangle = function(x, y, width, height)
        return {
            absolute = {
                x = x,
                y = y,
                width = width,
                height = height,
            },
        }
    end,
    ---Position the anchor on a Widget boundaries.
    ---@type fun(widget_id: string): snowcap.popup.Position
    AtWidget = function(widget_id)
        return {
            at_widget = widget_id,
        }
    end,
}
popup.position = position

---Position of the anchor point on the anchor rectangle.
---@enum snowcap.popup.Anchor
local anchor = {
    TOP = 1,
    BOTTOM = 2,
    LEFT = 3,
    RIGHT = 4,
    TOP_LEFT = 5,
    TOP_RIGHT = 6,
    BOTTOM_LEFT = 7,
    BOTTOM_RIGHT = 8,
    NONE = 9,
}
popup.anchor = anchor

---Direction of the gravity of the Popup.
---@enum snowcap.popup.Gravity
local gravity = {
    TOP = 1,
    BOTTOM = 2,
    LEFT = 3,
    RIGHT = 4,
    TOP_LEFT = 5,
    TOP_RIGHT = 6,
    BOTTOM_LEFT = 7,
    BOTTOM_RIGHT = 8,
    NONE = 9,
}
popup.gravity = gravity

---Popup position offset
---@class snowcap.popup.Offset
---@field x number
---@field y number

---Define ways the compositor can adjust the popup if its position would make it partially
---constrained.
---
---Except for none, every field are considered part of a bitfield.
---@class snowcap.popup.ConstraintsAdjust
---@field none? boolean Don't move the child surface when constrained.
---@field slide_x? boolean Move along the x axis until unconstrained.
---@field slide_y? boolean Move along the y axis until unconstrained.
---@field flip_x? boolean Invert the anchor and gravity on the x axis.
---@field flip_y? boolean Invert the anchor and gravity on the y axis.
---@field resize_x? boolean Horizontally resize the surface.
---@field resize_y? boolean Vertically resize the surface.

---popup.new_widget parameters.
---
---Only one parent handle will be taken into account. Setting more than one is undefined behavior.
---@class snowcap.popup.PopupArgs
---@field program snowcap.widget.Program Popup's content.
---@field parent snowcap.popup.ParentHandle Popup's parent surface handle.
---@field position snowcap.popup.Position Position the Popup should be placed at.
---@field anchor? snowcap.popup.Anchor Popup's anchor point on the Position boundaries.
---@field gravity? snowcap.popup.Gravity Popup's gravity.
---@field offset? snowcap.popup.Offset Popup's offset from the ancho point.
---@field constraints_adjust? snowcap.popup.ConstraintsAdjust Popup's constraints adjustment.
---@field no_grab? boolean If true, the Popup will not request an explicit keyboard grab upon creation.
---@field no_replace? boolean If true, the Popup will fail if there is already another popup with the same parent.

---@param args snowcap.popup.PopupArgs
---@return snowcap.popup.PopupHandle|nil handle A handle to the popup surface, or nil if an error occurred
function popup.new_widget(args)
    ---@type table<integer, any>
    local callbacks = {}

    local widget_def = args.program:view()

    widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

    ---@type snowcap.popup.v1.NewPopupRequest
    local request = {
        widget_def = widget.widget_def_into_api(widget_def),
        position = args.position --[[@as snowcap.popup.v1.Position]],
        anchor = args.anchor,
        gravity = args.gravity,
        offset = args.offset --[[@as snowcap.popup.v1.Offset]],
        constraints_adjust = args.constraints_adjust --[[@as snowcap.popup.v1.ConstraintsAdjust]],
        no_grab = args.no_grab,
        no_replace = args.no_replace,
    }

    assert(args.parent, "No ParentHandle")

    if args.parent.layer then
        request.layer_id = args.parent.layer
    elseif args.parent.decoration then
        request.deco_id = args.parent.decoration
    elseif args.parent.popup then
        request.popup_id = args.parent.popup
    else
        log.error("Parent surface missing.")
        return nil
    end

    local response, err = client:snowcap_popup_v1_PopupService_NewPopup(request)

    if err then
        log.error(err)
        return nil
    end

    assert(response)

    if not response.popup_id then
        log.error("no popup_id received")
        return nil
    end

    local popup_id = response.popup_id --[[@as integer]]

    ---@type fun(msg: any?)
    local update_on_msg = function(msg)
        if msg ~= nil then
            args.program:update(msg)
        end

        ---@diagnostic disable-next-line: redefined-local
        local _, err = client:snowcap_popup_v1_PopupService_RequestView({
            popup_id = popup_id,
        })

        if err then
            log.error(err)
        end
    end

    args.program:connect(widget_signal.redraw_needed, update_on_msg)
    args.program:connect(widget_signal.send_message, update_on_msg)

    err = client:snowcap_widget_v1_WidgetService_GetWidgetEvents({
        popup_id = popup_id,
    }, function(response) ---@diagnostic disable-line:redefined-local
        for _, event in ipairs(response.widget_events) do
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
        callbacks = {}

        widget._traverse_widget_tree(widget_def, callbacks, widget._collect_callbacks)

        ---@diagnostic disable-next-line:redefined-local
        local _, err = client:snowcap_popup_v1_PopupService_UpdatePopup({
            popup_id = popup_id,
            widget_def = widget.widget_def_into_api(widget_def),
        })

        if err then
            log.error(err)
        end
    end)

    return popup_handle.new(popup_id, update_on_msg)
end

---Do something when a key event is received.
---@param on_event fun(handle: snowcap.popup.PopupHandle, event: snowcap.input.KeyEvent)
function PopupHandle:on_key_event(on_event)
    local err = client:snowcap_input_v1_InputService_KeyboardKey(
        { popup_id = self.id },
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

---Do something on key press.
---@param on_press fun(mods: snowcap.input.Modifiers, key: snowcap.Key)
function PopupHandle:on_key_press(on_press)
    self:on_key_event(function(_, event)
        if not event.pressed or event.captured then
            return
        end

        on_press(event.mods, event.key)
    end)
end

---Sends an `Operation` to this popup.
---@param operation snowcap.widget.operation.Operation
function PopupHandle:operate(operation)
    local _, err = client:snowcap_popup_v1_PopupService_OperatePopup({
        popup_id = self.id,
        operation = require("snowcap.widget.operation")._to_api(operation), ---@diagnostic disable-line: invisible
    })

    if err then
        log.error(err)
    end
end

---PopupHandle:popup parameters.
---
---Any parameters set will override a previously set value.
---@class snowcap.popup.PopupUpdateArgs
---@field position snowcap.popup.Position? Update popup's position.
---@field anchor snowcap.popup.Anchor? Update popup's anchor.
---@field gravity snowcap.popup.Gravity? Update popup's gravity.
---@field offset snowcap.popup.Offset? Update popup's offset.
---@field constraints_adjust snowcap.popup.ConstraintsAdjust? Update popup's constraints adjustment.

---Update this popup's attributes.
---@param args snowcap.popup.PopupUpdateArgs
---@return boolean # True if the operation succeed.
function PopupHandle:update(args)
    local _, err = client:snowcap_popup_v1_PopupService_UpdatePopup({
        popup_id = self.id,
        position = args.position --[[@as snowcap.popup.v1.Position]],
        anchor = args.anchor --[[@as snowcap.popup.v1.Anchor]],
        gravity = args.gravity --[[@as snowcap.popup.v1.Gravity]],
        offset = args.offset --[[@as snowcap.popup.v1.Offset]],
        constraints_adjust = args.constraints_adjust --[[@as snowcap.popup.v1.ConstraintsAdjust]],
    })

    if err then
        log.error(err)
    end

    return err == nil
end

---Close this popup widget.
function PopupHandle:close()
    local _, err = client:snowcap_popup_v1_PopupService_Close({ popup_id = self.id })

    if err then
        log.error(err)
    end
end

---Sends a message to this Popup [`Program`].
function PopupHandle:send_message(message)
    self._update(message)
end

return popup
