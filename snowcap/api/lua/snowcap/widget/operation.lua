-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---Operation acting on widget that can be focused.
---@class snowcap.widget.operation.Focusable
---@field focus snowcap.widget.operation.focusable.Focus?
---@field unfocus snowcap.widget.operation.focusable.Unfocus?
---@field focus_next snowcap.widget.operation.focusable.FocusNext?
---@field focus_prev snowcap.widget.operation.focusable.FocusPrev?

---@class snowcap.widget.operation.focusable.Focus
---@field id string?

---@class snowcap.widget.operation.focusable.Unfocus
---@class snowcap.widget.operation.focusable.FocusNext
---@class snowcap.widget.operation.focusable.FocusPrev

---Operation acting on widget that have a text input.
---@class snowcap.widget.operation.TextInput
---@field move_cursor snowcap.widget.operation.text_input.MoveCursor?
---@field move_cursor_front snowcap.widget.operation.text_input.MoveCursorFront?
---@field move_cursor_end snowcap.widget.operation.text_input.MoveCursorEnd?
---@field select_all snowcap.widget.operation.text_input.SelectAll?

---@class snowcap.widget.operation.text_input.MoveCursor
---@field id string?
---@field position integer?

---@class snowcap.widget.operation.text_input.MoveCursorFront
---@field id string?

---@class snowcap.widget.operation.text_input.MoveCursorEnd
---@field id string?

---@class snowcap.widget.operation.text_input.SelectAll
---@field id string?

---Operation acting on WlrTaskList.
---@class snowcap.widget.operation.WlrTaskList
---@field maximize snowcap.widget.operation.wlr_task_list.MaximizeToplevel?
---@field minimize snowcap.widget.operation.wlr_task_list.MinimizeToplevel?
---@field fullscreen snowcap.widget.operation.wlr_task_list.FullscreenToplevel?
---@field activate snowcap.widget.operation.wlr_task_list.ActivateToplevel?
---@field close snowcap.widget.operation.wlr_task_list.CloseToplevel?

---@class snowcap.widget.operation.wlr_task_list.MaximizeToplevel
---@field id integer
---@field maximize boolean

---@class snowcap.widget.operation.wlr_task_list.MinimizeToplevel
---@field id integer
---@field minimize boolean

---@class snowcap.widget.operation.wlr_task_list.FullscreenToplevel
---@field id integer
---@field fullscreen boolean

---@class snowcap.widget.operation.wlr_task_list.ActivateToplevel
---@field id integer

---@class snowcap.widget.operation.wlr_task_list.CloseToplevel
---@field id integer

---Update widgets' internal state.
---@class snowcap.widget.operation.Operation
---@field focusable snowcap.widget.operation.Focusable?
---@field text_input snowcap.widget.operation.TextInput?
---@field wlr_task_list snowcap.widget.operation.WlrTaskList?

---Operation acting on widgets that can be focused.
---@class snowcap.widget.operation.focusable
local focusable = {
    ---Operation to remove focus from any widget.
    ---@type snowcap.widget.operation.Operation
    Unfocus = { focusable = { unfocus = {} } },

    ---Operation to focus the next widget in the tree, or the first one.
    ---@type snowcap.widget.operation.Operation
    FocusNext = { focusable = { focus_next = {} } },

    ---Operation to focus the previous widget in the tree, or the last one.
    ---@type snowcap.widget.operation.Operation
    FocusPrev = { focusable = { focus_prev = {} } },
}

---Operation to focus a specific widget.
---
---@param widget_id string Widget's id, as specified by its `id` field.
---
---@return snowcap.widget.operation.Operation
function focusable.Focus(widget_id)
    ---@type snowcap.widget.operation.Operation
    return {
        focusable = {
            focus = { id = widget_id },
        },
    }
end

---Operates on widgets that have text input.
---@class snowcap.widget.operation.text_input
local text_input = {}

---Operation that moves the cursor to a specified position.
---@param widget_id string Widget's Id, as specified by its `id` field.
---@param position integer Position to set the cursor to.
---
---@return snowcap.widget.operation.Operation
function text_input.MoveCursor(widget_id, position)
    ---@type snowcap.widget.operation.Operation
    return {
        text_input = {
            move_cursor = { id = widget_id, position = position },
        },
    }
end

---Operation that moves the cursor to the start of the field.
---@param widget_id string Widget's Id, as specified by its `id` field.
---
---@return snowcap.widget.operation.Operation
function text_input.MoveCursorFront(widget_id)
    ---@type snowcap.widget.operation.Operation
    return {
        text_input = {
            move_cursor_front = { id = widget_id },
        },
    }
end

---Operation that moves the widget's cursor to the end of the field.
---@param widget_id string Widget's Id, as specified by its `id` field.
---
---@return snowcap.widget.operation.Operation
function text_input.MoveCursorEnd(widget_id)
    ---@type snowcap.widget.operation.Operation
    return {
        text_input = {
            move_cursor_end = { id = widget_id },
        },
    }
end

---Operation that select the full content of a widget.
---@param widget_id string Widget's Id, as specified by its `id` field.
---
---@return snowcap.widget.operation.Operation
function text_input.SelectAll(widget_id)
    ---@type snowcap.widget.operation.Operation
    return {
        text_input = {
            select_all = { id = widget_id },
        },
    }
end

---Operates on WlrTaskList
---@class snowcap.widget.operation.wlr_task_list
local wlr_task_list = {}

---Operation that request a toplevel to be maximized or unmaximized.
---@param toplevel_id integer Topleve Id.
---@param maximize boolean
---
---@return snowcap.widget.operation.Operation
function wlr_task_list.MaximizeToplevel(toplevel_id, maximize)
    ---@type snowcap.widget.operation.Operation
    return {
        wlr_task_list = {
            maximize = {
                id = toplevel_id,
                maximize = maximize
            }
        }
    }
end

---Operation that request a toplevel to be minimized or unminimized.
---@param toplevel_id integer Topleve Id.
---@param minimize boolean
---
---@return snowcap.widget.operation.Operation
function wlr_task_list.MinimizeToplevel(toplevel_id, minimize)
    ---@type snowcap.widget.operation.Operation
    return {
        wlr_task_list = {
            minimize = {
                id = toplevel_id,
                minimize = minimize
            }
        }
    }
end

---Operation that request a toplevel to be fullscreened or unfullscreened.
---@param toplevel_id integer Topleve Id.
---@param fullscreen boolean
---
---@return snowcap.widget.operation.Operation
function wlr_task_list.FullscreenToplevel(toplevel_id, fullscreen)
    ---@type snowcap.widget.operation.Operation
    return {
        wlr_task_list = {
            fullscreen = {
                id = toplevel_id,
                fullscreen = fullscreen
            }
        }
    }
end

---Operation that request a toplevel to be activated.
---@param toplevel_id integer Toplevel Id.
---
---@return snowcap.widget.operation.Operation
function wlr_task_list.ActivateToplevel(toplevel_id)
    ---@type snowcap.widget.operation.Operation
    return {
        wlr_task_list = {
            activate = {
                id = toplevel_id,
            }
        }
    }
end

---Operation that request a toplevel to be closed.
---@param toplevel_id integer Toplevel Id
---
---@return snowcap.widget.operation.Operation
function wlr_task_list.CloseToplevel(toplevel_id)
    ---@type snowcap.widget.operation.Operation
    return {
        wlr_task_list = {
            close = {
                id = toplevel_id,
            }
        }
    }
end

---Update internal state for some widgets.
---
---`Operation` can be passed to `LayerHandle:operate` and `DecorationHandle::operate` to
---act on their widgets states.
---
---## Example
---Focus a given widget:
---```lua
---local Operation = require("snowcap.widget.operation")
---
---function focus_widget(handle, widget_id)
---    handle:operate(Operation.focusable.Focus(widget_id))
---end
---```
---
---Focus a widget and move the cursor to the beginning of the field:
---```lua
---local Operation = require("snowcap.widget.operation")
---
---function focus_widget(handle, widget_id)
---    handle:operate(Operation.focusable.Focus(widget_id))
---    handle:operate(Operation.text_input.MoveCursorFront(widget_id))
---end
---```
---@class snowcap.widget.operation
---
---Operations acting on widget that can be focused.
---@field focusable snowcap.widget.operation.focusable
---Operations acting on widget that have a text input.
---@field text_input snowcap.widget.operation.text_input
local operation = {
    focusable = focusable,
    text_input = text_input,
    wlr_task_list = wlr_task_list,
}

---@private
---@lcat nodoc
---@param op snowcap.widget.operation.Operation
---@return snowcap.operation.v1.Operation
function operation._to_api(op)
    return op --[[@as snowcap.operation.v1.Operation]]
end

return operation
