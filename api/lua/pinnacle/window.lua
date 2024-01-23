-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---The protobuf absolute path prefix
local prefix = "pinnacle.window." .. require("pinnacle").version .. "."
local service = prefix .. "WindowService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) WindowServiceMethod
local rpc_types = {
    Close = {},
    SetGeometry = {},
    SetFullscreen = {},
    SetMaximized = {},
    SetFloating = {},
    MoveToTag = {},
    SetTag = {},
    MoveGrab = {},
    ResizeGrab = {},
    Get = {
        response_type = "GetResponse",
    },
    GetProperties = {
        response_type = "GetPropertiesResponse",
    },
    AddWindowRule = {},
}

---Build GrpcRequestParams
---@param method WindowServiceMethod
---@param data table
---@return GrpcRequestParams
local function build_grpc_request_params(method, data)
    local req_type = rpc_types[method].request_type
    local resp_type = rpc_types[method].response_type

    ---@type GrpcRequestParams
    return {
        service = service,
        method = method,
        request_type = req_type and prefix .. req_type or prefix .. method .. "Request",
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---@nodoc
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
---@classmod
---@class WindowHandle
---@field private config_client Client
---@field id integer
local WindowHandle = {}

---@nodoc
---@class WindowModule
---@field private handle WindowHandleModule
local window = {}
window.handle = window_handle

---Window management.
---
---This module helps you deal with setting windows to fullscreen and maximized, setting their size,
---moving them between tags, and various other actions.
---@class Window
---@field private config_client Client
local Window = {}

---Get all windows.
---
---### Example
---```lua
---local windows = Window:get_all()
---for _, window in ipairs(windows) do
---    print(window:props().class)
---end
---```
---@return WindowHandle[] windows Handles to all windows
function Window:get_all()
    local response = self.config_client:unary_request(build_grpc_request_params("Get", {}))

    local handles = window_handle.new_from_table(self.config_client, response.window_ids or {})

    return handles
end

---Get the currently focused window.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    print(focused:props().class)
---end
---```
---@return WindowHandle | nil window A handle to the currently focused window
function Window:get_focused()
    local handles = self:get_all()

    for _, handle in ipairs(handles) do
        if handle:props().focused then
            return handle
        end
    end

    return nil
end

---Begin moving this window using the specified mouse button.
---
---The button must be pressed at the time this method is called.
---If the button is lifted, the move will end.
---
---### Example
---```lua
---Input:mousebind({ "super" }, "btn_left", function()
---    Window:begin_move("btn_left")
---end)
---```
---@param button MouseButton The button that will initiate the move
function Window:begin_move(button)
    local button = require("pinnacle.input").btn[button]
    self.config_client:unary_request(build_grpc_request_params("MoveGrab", { button = button }))
end

---Begin resizing this window using the specified mouse button.
---
---The button must be pressed at the time this method is called.
---If the button is lifted, the resize will end.
---
---### Example
---```lua
---Input:mousebind({ "super" }, "btn_right", function()
---    Window:begin_resize("btn_right")
---end)
---```
---@param button MouseButton The button that will initiate the resize
function Window:begin_resize(button)
    local button = require("pinnacle.input").btn[button]
    self.config_client:unary_request(build_grpc_request_params("ResizeGrab", { button = button }))
end

---@class WindowRuleCondition
---@field any WindowRuleCondition[]?
---@field all WindowRuleCondition[]?
---@field classes string[]?
---@field titles string[]?
---@field tags TagHandle[]?

---@class WindowRule
---@field output OutputHandle?
---@field tags TagHandle[]?
---@field floating boolean?
---@field fullscreen_or_maximized FullscreenOrMaximized?
---@field x integer?
---@field y integer?
---@field width integer?
---@field height integer?

---@enum (key) FullscreenOrMaximized
local _fullscreen_or_maximized = {
    neither = 1,
    fullscreen = 2,
    maximized = 3,
}

local _fullscreen_or_maximized_keys = {
    [1] = "neither",
    [2] = "fullscreen",
    [3] = "maximized",
}

---Add a window rule.
---
---A window rule defines what properties a window will spawn with given certain conditions.
---For example, if Firefox is spawned, you can set it to open on a specific tag.
---
---This method takes in a table with two keys:
---
--- - `cond`: The condition for `rule` to apply to a new window.
--- - `rule`: What gets applied to the new window if `cond` is true.
---
---There are some important mechanics you should know when using window rules:
---
--- - All children inside an `all` block must be true for the block to be true.
--- - At least one child inside an `any` block must be true for the block to be true.
--- - The outermost block of a window rule condition is implicitly an `all` block.
--- - Within an `all` block, all items in each array must be true for the attribute to be true.
--- - Within an `any` block, only one item in each array needs to be true for the attribute to be true.
---
---`cond` can be a bit confusing and quite table heavy. Examples are shown below for guidance.
---
---### Examples
---```lua
--- -- A simple window rule. This one will cause Firefox to open on tag "Browser".
---Window:add_window_rule({
---    cond = { classes = { "firefox" } },
---    rule = { tags = { "Browser" } },
---})
---
--- -- To apply rules when *all* provided conditions are true, use `all`.
--- -- `all` takes an array of conditions and checks if all are true.
--- -- The following will open Steam fullscreen only if it opens on tag "5".
---Window:add_window_rule({
---    cond = {
---        all = {
---            {
---                class = "steam",
---                tag = Tag:get("5"),
---            }
---        }
---    },
---    rule = { fullscreen_or_maximized = "fullscreen" },
---})
---
--- -- The outermost block of a `cond` is implicitly an `all` block.
--- -- Thus, the above can be shortened to:
---Window:add_window_rule({
---    cond = {
---        class = "steam",
---        tag = Tag:get("5"),
---    },
---    rule = { fullscreen_or_maximized = "fullscreen" },
---})
---
--- -- `any` also exists to allow at least one provided condition to match.
--- -- The following will open either xterm or Alacritty floating.
---Window:add_window_rule({
---    cond = {
---        any = { { classes = { "xterm", "Alacritty" } } }
---    },
---    rule = { floating = true },
---})
---
--- -- You can arbitrarily nest `any` and `all` to achieve desired logic.
--- -- The following will open Discord, Thunderbird, or Firefox floating if they
--- -- open on either *all* of tags "A", "B", and "C" or both tags "1" and "2".
---Window:add_window_rule({
---    cond = {
---        all = { -- This `all` block is needed because the outermost block cannot be an array.
---            { any = {
---                { class = { "firefox", "thunderbird", "discord" } }
---            } },
---            { any = {
---                -- Because `tag` is inside an `all` block,
---                -- the window must have all these tags for this to be true.
---                -- If it was in an `any` block, only one tag would need to match.
---                { all = {
---                    { tag = { "A", "B", "C" } }
---                } },
---                { all = {
---                    { tag = { "1", "2" } }
---                } },
---            } }
---        }
---    },
---    rule = { floating = true },
---})
---```
---
---@param rule { cond: WindowRuleCondition, rule: WindowRule } The condition and rule
function Window:add_window_rule(rule)
    if rule.cond.tags then
        local ids = {}
        for _, tg in ipairs(rule.cond.tags) do
            table.insert(ids, tg.id)
        end
        rule.cond.tags = ids
    end

    if rule.rule.output then
        rule.rule.output = rule.rule.output.name
    end

    if rule.rule.tags then
        local ids = {}
        for _, tg in ipairs(rule.rule.tags) do
            table.insert(ids, tg.id)
        end
        rule.rule.tags = ids
    end

    if rule.rule.fullscreen_or_maximized then
        rule.rule.fullscreen_or_maximized = _fullscreen_or_maximized[rule.rule.fullscreen_or_maximized]
    end

    self.config_client:unary_request(build_grpc_request_params("AddWindowRule", {
        cond = rule.cond,
        rule = rule.rule,
    }))
end

---Send a close request to this window.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then focused:close() end
---```
function WindowHandle:close()
    self.config_client:unary_request(build_grpc_request_params("Close", { window_id = self.id }))
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
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:set_floating(true)                     -- `set_geometry` only applies to floating geometry.
---
---    focused:set_geometry({ x = 50, y = 300 })      -- Move this window to (50, 300)
---    focused:set_geometry({ y = 0, height = 1080 }) -- Move this window to y = 0 and make its height 1080 pixels
---    focused:set_geometry({})                       -- Do nothing useful
---end
---```
---@param geo { x: integer?, y: integer, width: integer?, height: integer? } The new location and/or size
function WindowHandle:set_geometry(geo)
    self.config_client:unary_request(build_grpc_request_params("SetGeometry", { window_id = self.id, geometry = geo }))
end

---Set this window to fullscreen or not.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:set_fullscreen(true)
---    focused:set_fullscreen(false)
---end
---```
---
---@param fullscreen boolean
function WindowHandle:set_fullscreen(fullscreen)
    self.config_client:unary_request(
        build_grpc_request_params("SetFullscreen", { window_id = self.id, set = fullscreen })
    )
end

---Toggle this window to and from fullscreen.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:toggle_fullscreen()
---end
---```
function WindowHandle:toggle_fullscreen()
    self.config_client:unary_request(build_grpc_request_params("SetFullscreen", { window_id = self.id, toggle = {} }))
end

---Set this window to maximized or not.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:set_maximized(true)
---    focused:set_maximized(false)
---end
---```
---
---@param maximized boolean
function WindowHandle:set_maximized(maximized)
    self.config_client:unary_request(
        build_grpc_request_params("SetMaximized", { window_id = self.id, set = maximized })
    )
end

---Toggle this window to and from maximized.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:toggle_maximized()
---end
---```
function WindowHandle:toggle_maximized()
    self.config_client:unary_request(build_grpc_request_params("SetMaximized", { window_id = self.id, toggle = {} }))
end

---Set this window to floating or not.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:set_floating(true)
---    focused:set_floating(false)
---end
---```
---
---@param floating boolean
function WindowHandle:set_floating(floating)
    self.config_client:unary_request(build_grpc_request_params("SetFloating", { window_id = self.id, set = floating }))
end

---Toggle this window to and from floating.
---
---### Example
---```lua
---local focused = Window:get_focused()
---if focused then
---    focused:toggle_floating()
---end
---```
function WindowHandle:toggle_floating()
    self.config_client:unary_request(build_grpc_request_params("SetFloating", { window_id = self.id, toggle = {} }))
end

---Move this window to the specified tag.
---
---This will remove all tags from this window and tag it with `tag`.
---
---### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window:get_focused()
---if focused then
---    focused:move_to_tag(Tag:get("Tag"))
---end
---```
---
---@param tag TagHandle The tag to move this window to
function WindowHandle:move_to_tag(tag)
    self.config_client:unary_request(build_grpc_request_params("MoveToTag", { window_id = self.id, tag_id = tag.id }))
end

---Tag or untag the given tag on this window.
---
---### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window:get_focused()
---if focused then
---    local tag = Tag:get("Tag")
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
    self.config_client:unary_request(
        build_grpc_request_params("SetTag", { window_id = self.id, tag_id = tag.id, set = set })
    )
end

---Toggle the given tag on this window.
---
---### Example
---```lua
--- -- Assume the focused output has the tag "Tag"
---local focused = Window:get_focused()
---if focused then
---    local tag = Tag:get("Tag")
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
    self.config_client:unary_request(
        build_grpc_request_params("SetTag", { window_id = self.id, tag_id = tag.id, toggle = {} })
    )
end

---@class WindowProperties
---@field geometry { x: integer?, y: integer?, width: integer?, height: integer? }? The location and size of the window
---@field class string? The window's class
---@field title string? The window's title
---@field focused boolean? Whether or not the window is focused
---@field floating boolean? Whether or not the window is floating
---@field fullscreen_or_maximized FullscreenOrMaximized? Whether the window is fullscreen, maximized, or neither
---@field tags TagHandle[]? The tags the window has

---Get all the properties of this window.
---
---@return WindowProperties
function WindowHandle:props()
    local response =
        self.config_client:unary_request(build_grpc_request_params("GetProperties", { window_id = self.id }))

    response.fullscreen_or_maximized = _fullscreen_or_maximized_keys[response.fullscreen_or_maximized]

    response.tags = response.tag_ids
        and require("pinnacle.tag").handle.new_from_table(self.config_client, response.tag_ids)
    response.tag_ids = nil

    return response
end

---Get this window's location and size.
---
---Shorthand for `handle:props().geometry`.
---
---@return { x: integer?, y: integer?, width: integer?, height: integer? }?
function WindowHandle:geometry()
    return self:props().geometry
end

---Get this window's class.
---
---Shorthand for `handle:props().class`.
---
---@return string?
function WindowHandle:class()
    return self:props().class
end

---Get this window's title.
---
---Shorthand for `handle:props().title`.
---
---@return string?
function WindowHandle:title()
    return self:props().title
end

---Get whether or not this window is focused.
---
---Shorthand for `handle:props().focused`.
---
---@return boolean?
function WindowHandle:focused()
    return self:props().focused
end

---Get whether or not this window is floating.
---
---Shorthand for `handle:props().floating`.
---
---@return boolean?
function WindowHandle:floating()
    return self:props().floating
end

---Get whether this window is fullscreen, maximized, or neither.
---
---Shorthand for `handle:props().fullscreen_or_maximized`.
---
---@return FullscreenOrMaximized?
function WindowHandle:fullscreen_or_maximized()
    return self:props().fullscreen_or_maximized
end

---Get all tags on this window.
---
---Shorthand for `handle:props().tags`.
---
---@return TagHandle[]?
function WindowHandle:tags()
    return self:props().tags
end

---@nodoc
---@param config_client Client
---@return Window
function window.new(config_client)
    ---@type Window
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Window })
    return self
end

---@nodoc
---Create a new `WindowHandle` from an id.
---@param config_client Client
---@param window_id integer
---@return WindowHandle
function window_handle.new(config_client, window_id)
    ---@type WindowHandle
    local self = {
        config_client = config_client,
        id = window_id,
    }
    setmetatable(self, { __index = WindowHandle })
    return self
end

---@nodoc
---@param config_client Client
---@param window_ids integer[]
---
---@return WindowHandle[]
function window_handle.new_from_table(config_client, window_ids)
    ---@type WindowHandle[]
    local handles = {}

    for _, id in ipairs(window_ids) do
        table.insert(handles, window_handle.new(config_client, id))
    end

    return handles
end

return window
