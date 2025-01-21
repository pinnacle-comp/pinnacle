-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local window_v1 = require("pinnacle.grpc.defs").pinnacle.window.v1
local window_service = require("pinnacle.grpc.defs").pinnacle.window.v1.WindowService
local defs = require("pinnacle.grpc.defs")

local set_or_toggle = {
    SET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    [true] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_SET,
    UNSET = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    [false] = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    TOGGLE = require("pinnacle.grpc.defs").pinnacle.util.v1.SetOrToggle.SET_OR_TOGGLE_TOGGLE,
}

local layout_mode_def = require("pinnacle.grpc.defs").pinnacle.window.v1.LayoutMode

---@lcat nodoc
---@class WindowHandleModule
local window_handle = {}

---A window handle.
---
---This is a handle to an application window that allows manipulation of the window.
---
---If the window is destroyed, the handle will become invalid and may not do
---what you want it to.
---
---You can retrieve window handles through the various `get` functions in the `Window` module.
---@class WindowHandle
---@field id integer
local WindowHandle = {}

---Window management.
---
---This module helps you deal with setting windows to fullscreen and maximized, setting their size,
---moving them between tags, and various other actions.
---@class Window
---@lcat nodoc
---@field private handle WindowHandleModule
local window = {}
window.handle = window_handle

---Get all windows.
---
---#### Example
---```lua
---local windows = Window.get_all()
---for _, window in ipairs(windows) do
---    print(window:props().class)
---end
---```
---@return WindowHandle[] windows Handles to all windows
function window.get_all()
    local response, err = client:unary_request(window_service.Get, {})

    if err then
        log:error(err)
        return {}
    end

    ---@cast response pinnacle.window.v1.GetResponse

    local handles = window_handle.new_from_table(response.window_ids or {})

    return handles
end

---Get the currently focused window.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    print(focused:props().class)
---end
---```
---@return WindowHandle | nil window A handle to the currently focused window
function window.get_focused()
    local handles = window.get_all()

    ---@type (fun(): bool)[]
    local requests = {}

    for i, handle in ipairs(handles) do
        requests[i] = function()
            return handle:focused()
        end
    end

    local props = require("pinnacle.util").batch(requests)

    for i, focused in ipairs(props) do
        if focused then
            return handles[i]
        end
    end

    return nil
end

---Begin moving this window using the specified mouse button.
---
---The button must be pressed at the time this method is called.
---If the button is lifted, the move will end.
---
---#### Example
---```lua
---Input.mousebind({ "super" }, "btn_left", function()
---    Window.begin_move("btn_left")
---end)
---```
---@param button MouseButton The button that will initiate the move
function window.begin_move(button)
    ---@diagnostic disable-next-line: redefined-local, invisible
    local button = require("pinnacle.input").mouse_button_values[button]
    local _, err = client:unary_request(window_service.MoveGrab, { button = button })

    if err then
        log:error(err)
    end
end

---Begin resizing this window using the specified mouse button.
---
---The button must be pressed at the time this method is called.
---If the button is lifted, the resize will end.
---
---#### Example
---```lua
---Input.mousebind({ "super" }, "btn_right", function()
---    Window.begin_resize("btn_right")
---end)
---```
---@param button MouseButton The button that will initiate the resize
function window.begin_resize(button)
    ---@diagnostic disable-next-line: redefined-local, invisible
    local button = require("pinnacle.input").mouse_button_values[button]
    local _, err = client:unary_request(window_service.ResizeGrab, { button = button })

    if err then
        log:error(err)
    end
end

---@enum (key) LayoutMode
local layout_mode = {
    tiled = window_v1.LayoutMode.LAYOUT_MODE_TILED,
    floating = window_v1.LayoutMode.LAYOUT_MODE_FLOATING,
    fullscreen = window_v1.LayoutMode.LAYOUT_MODE_FULLSCREEN,
    maximized = window_v1.LayoutMode.LAYOUT_MODE_MAXIMIZED,
}
require("pinnacle.util").make_bijective(layout_mode)

local signal_name_to_SignalName = {
    pointer_enter = "WindowPointerEnter",
    pointer_leave = "WindowPointerLeave",
}

---@class WindowSignal Signals related to compositor events.
---@field pointer_enter fun(window: WindowHandle)? The pointer entered a window.
---@field pointer_leave fun(window: WindowHandle)? The pointer left a window.

---Connect to a window signal.
---
---The compositor sends signals about various events. Use this function to run a callback when
---some window signal occurs.
---
---This function returns a table of signal handles with each handle stored at the same key used
---to connect to the signal. See `SignalHandles` for more information.
---
---# Example
---```lua
---Window.connect_signal({
---    pointer_enter = function(window)
---        print("Pointer entered", window:class())
---    end
---})
---```
---
---@param signals WindowSignal The signal you want to connect to
---
---@return SignalHandles signal_handles Handles to every signal you connected to wrapped in a table, with keys being the same as the connected signal.
---
---@see SignalHandles.disconnect_all - To disconnect from these signals
function window.connect_signal(signals)
    ---@diagnostic disable-next-line: invisible
    local handles = require("pinnacle.signal").handles.new({})

    for signal, callback in pairs(signals) do
        require("pinnacle.signal").add_callback(signal_name_to_SignalName[signal], callback)
        local handle =
            ---@diagnostic disable-next-line: invisible
            require("pinnacle.signal").handle.new(signal_name_to_SignalName[signal], callback)
        handles[signal] = handle
    end

    return handles
end

---@param for_each fun(window: WindowHandle)
function window.for_each_window(for_each)
    local _stream, err = client:bidirectional_streaming_request(
        window_service.WindowRule,
        function(response, stream)
            local handle = window_handle.new(response.new_window.window_id)

            for_each(handle)

            local chunk =
                require("pinnacle.grpc.protobuf").encode("pinnacle.window.v1.WindowRuleRequest", {
                    finished = {
                        request_id = response.new_window.request_id,
                    },
                })

            local success, err = pcall(stream.write_chunk, stream, chunk)

            if not success then
                print("error sending to stream:", err)
            end
        end
    )

    if err then
        log:error("failed to start bidir stream")
        os.exit(1)
    end
end

------------------------------------------------------------------------

---Send a close request to this window.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then focused:close() end
---```
function WindowHandle:close()
    local _, err = client:unary_request(window_service.Close, { window_id = self.id })

    if err then
        log:error(err)
    end
end

---Set this window's location and/or size.
---
---The coordinate system has the following axes:
---```
---       ^ -y
---       |
--- -x <--+--> +x
---       |
---       v +y
---```
---
---*Tiled windows will not reflect these changes.*
---This method only applies to this window's floating geometry.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:set_floating(true)                     -- `set_geometry` only applies to floating geometry.
---
---    focused:set_geometry({ x = 50, y = 300 })      -- Move this window to (50, 300)
---    focused:set_geometry({ y = 0, height = 1080 }) -- Move this window to y = 0 and make its height 1080 pixels
---    focused:set_geometry({})                       -- Do nothing useful
---end
---```
---@param geo { x: integer?, y: integer?, width: integer?, height: integer? } The new location and/or size
function WindowHandle:set_geometry(geo)
    local _, err = client:unary_request(
        window_service.SetGeometry,
        { window_id = self.id, x = geo.x, y = geo.y, w = geo.width, h = geo.height }
    )

    if err then
        log:error(err)
    end
end

---Set this window to fullscreen or not.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:set_fullscreen(true)
---    focused:set_fullscreen(false)
---end
---```
---
---@param fullscreen boolean
function WindowHandle:set_fullscreen(fullscreen)
    local _, err = client:unary_request(
        window_service.SetFullscreen,
        { window_id = self.id, set_or_toggle = set_or_toggle[fullscreen] }
    )

    if err then
        log:error(err)
    end
end

---Toggle this window to and from fullscreen.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:toggle_fullscreen()
---end
---```
function WindowHandle:toggle_fullscreen()
    local _, err = client:unary_request(
        window_service.SetFullscreen,
        { window_id = self.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---Set this window to maximized or not.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:set_maximized(true)
---    focused:set_maximized(false)
---end
---```
---
---@param maximized boolean
function WindowHandle:set_maximized(maximized)
    local _, err = client:unary_request(
        window_service.SetMaximized,
        { window_id = self.id, set_or_toggle = set_or_toggle[maximized] }
    )

    if err then
        log:error(err)
    end
end

---Toggle this window to and from maximized.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:toggle_maximized()
---end
---```
function WindowHandle:toggle_maximized()
    local _, err = client:unary_request(
        window_service.SetMaximized,
        { window_id = self.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---Set this window to floating or not.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:set_floating(true)
---    focused:set_floating(false)
---end
---```
---
---@param floating boolean
function WindowHandle:set_floating(floating)
    local _, err = client:unary_request(
        window_service.SetFloating,
        { window_id = self.id, set_or_toggle = set_or_toggle[floating] }
    )

    if err then
        log:error(err)
    end
end

---Toggle this window to and from floating.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:toggle_floating()
---end
---```
function WindowHandle:toggle_floating()
    local _, err = client:unary_request(
        window_service.SetFloating,
        { window_id = self.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---Focus or unfocus this window.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:set_focused(false)
---end
---```
---
---@param focused boolean
function WindowHandle:set_focused(focused)
    local _, err = client:unary_request(
        window_service.SetFocused,
        { window_id = self.id, set_or_toggle = set_or_toggle[focused] }
    )

    if err then
        log:error(err)
    end
end

---Toggle this window to and from focused.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:toggle_focused()
---end
---```
function WindowHandle:toggle_focused()
    local _, err = client:unary_request(
        window_service.SetFocused,
        { window_id = self.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---@param mode "client_side" | "server_side"
function WindowHandle:set_decoration_mode(mode)
    local _, err = client:unary_request(
        window_service.SetDecorationMode,
        {
            window_id = self.id,
            decoration_mode = mode == "client_side"
                    and defs.pinnacle.window.v1.DecorationMode.DECORATION_MODE_CLIENT_SIDE
                or defs.pinnacle.window.v1.DecorationMode.DECORATION_MODE_SERVER_SIDE,
        }
    )

    if err then
        log:error(err)
    end
end

---Move this window to the specified tag.
---
---This will remove all tags from this window and tag it with `tag`.
---
---#### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window.get_focused()
---if focused then
---    focused:move_to_tag(Tag.get("Tag"))
---end
---```
---
---@param tag TagHandle The tag to move this window to
function WindowHandle:move_to_tag(tag)
    local _, err =
        client:unary_request(window_service.MoveToTag, { window_id = self.id, tag_id = tag.id })

    if err then
        log:error(err)
    end
end

---Tag or untag the given tag on this window.
---
---#### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window.get_focused()
---if focused then
---    local tag = Tag.get("Tag")
---
---    focused:set_tag(tag, true)
---    -- `focused` now has tag "Tag"
---    focused:set_tag(tag, false)
---    -- `focused` no longer has tag "Tag"
---end
---```
---
---@param tag TagHandle The tag to set or unset
---@param set boolean
function WindowHandle:set_tag(tag, set)
    local _, err = client:unary_request(
        window_service.SetTag,
        { window_id = self.id, tag_id = tag.id, set_or_toggle = set_or_toggle[set] }
    )

    if err then
        log:error(err)
    end
end

---Toggle the given tag on this window.
---
---#### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window.get_focused()
---if focused then
---    local tag = Tag.get("Tag")
---    focused:set_tag(tag, false)
---
---    focused:toggle_tag(tag)
---    -- `focused` now has tag "Tag"
---    focused:toggle_tag(tag)
---    -- `focused` no longer has tag "Tag"
---end
---```
---
---@param tag TagHandle The tag to toggle
function WindowHandle:toggle_tag(tag)
    local _, err = client:unary_request(
        window_service.SetTag,
        { window_id = self.id, tag_id = tag.id, set_or_toggle = set_or_toggle.TOGGLE }
    )

    if err then
        log:error(err)
    end
end

---Raise a window.
---
---This will raise a window all the way to the top of the z-stack.
---
---#### Example
---```lua
---local focused = Window.get_focused()
---if focused then
---    focused:raise()
---end
---```
function WindowHandle:raise()
    local _, err = client:unary_request(window_service.Raise, { window_id = self.id })

    if err then
        log:error(err)
    end
end

---Returns whether or not this window is on an active tag.
---
---@return boolean
function WindowHandle:is_on_active_tag()
    local tags = self:tags() or {}

    ---@type (fun(): boolean)[]
    local batch = {}

    for i, tg in ipairs(tags) do
        batch[i] = function()
            return tg:active() or false
        end
    end

    local actives = require("pinnacle.util").batch(batch)

    for _, active in ipairs(actives) do
        if active then
            return true
        end
    end

    return false
end

---Get this window's location and size.
---
---@return { x: integer, y: integer }?
function WindowHandle:loc()
    local loc, err = client:unary_request(window_service.GetLoc, { window_id = self.id })

    ---@cast loc pinnacle.window.v1.GetLocResponse|nil

    return loc and loc.loc
end

---Get this window's class.
---
---Shorthand for `handle:props().class`.
---
---@return string
function WindowHandle:app_id()
    local response, err = client:unary_request(window_service.GetAppId, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetAppIdResponse|nil

    return response and response.app_id
end

---Get this window's title.
---
---Shorthand for `handle:props().title`.
---
---@return string
function WindowHandle:title()
    local response, err = client:unary_request(window_service.GetTitle, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetTitleResponse|nil

    return response and response.title
end

---Get whether or not this window is focused.
---
---Shorthand for `handle:props().focused`.
---
---@return boolean
function WindowHandle:focused()
    local response, err = client:unary_request(window_service.GetFocused, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetFocusedResponse|nil

    return response and response.focused
end

---Get whether or not this window is floating.
---
---Shorthand for `handle:props().floating`.
---
---@return boolean
function WindowHandle:floating()
    local response, err =
        client:unary_request(window_service.GetLayoutMode, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetLayoutModeResponse|nil

    return response and response.layout_mode == layout_mode_def.LAYOUT_MODE_FLOATING or false
end

---Get whether this window is tiled.
---
---@return boolean
function WindowHandle:tiled()
    local response, err =
        client:unary_request(window_service.GetLayoutMode, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetLayoutModeResponse|nil

    return response and response.layout_mode == layout_mode_def.LAYOUT_MODE_TILED or false
end

---Get whether this window is fullscreen.
---
---@return boolean
function WindowHandle:fullscreen()
    local response, err =
        client:unary_request(window_service.GetLayoutMode, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetLayoutModeResponse|nil

    return response and response.layout_mode == layout_mode_def.LAYOUT_MODE_FULLSCREEN or false
end

---Get whether this window is maximized.
---
---@return boolean
function WindowHandle:maximized()
    local response, err =
        client:unary_request(window_service.GetLayoutMode, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetLayoutModeResponse|nil

    return response and response.layout_mode == layout_mode_def.LAYOUT_MODE_MAXIMIZED or false
end

---Get all tags on this window.
---
---Shorthand for `handle:props().tags`.
---
---@return TagHandle[]
function WindowHandle:tags()
    local response, err = client:unary_request(window_service.GetTagIds, { window_id = self.id })

    ---@cast response pinnacle.window.v1.GetTagIdsResponse|nil

    local tag_ids = response and response.tag_ids or {}

    local handles = require("pinnacle.tag").handle.new_from_table(tag_ids)

    return handles
end

---Create a new `WindowHandle` from an id.
---@param window_id integer
---@return WindowHandle
function window_handle.new(window_id)
    ---@type WindowHandle
    local self = {
        id = window_id,
    }
    setmetatable(self, { __index = WindowHandle })
    return self
end

---@param window_ids integer[]
---
---@return WindowHandle[]
function window_handle.new_from_table(window_ids)
    ---@type WindowHandle[]
    local handles = {}

    for _, id in ipairs(window_ids) do
        table.insert(handles, window_handle.new(id))
    end

    return handles
end

return window
