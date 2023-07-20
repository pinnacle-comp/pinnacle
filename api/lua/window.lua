-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@class Window
---@field private id integer The internal id of this window
local win = {}

---@param props Window
---@return Window
local function new_window(props)
    -- Copy functions over
    for k, v in pairs(win) do
        props[k] = v
    end

    return props
end

---Set a window's size.
---
---### Examples
---```lua
---window.get_focused():set_size({ w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---window.get_focused():set_size({ h = 300 })          -- keep the window's width but make it 300 pixels tall
---window.get_focused():set_size({})                   -- do absolutely nothing useful
---```
---@param size { w: integer?, h: integer? }
function win:set_size(size)
    SendMsg({
        SetWindowSize = {
            window_id = self.id,
            width = size.w,
            height = size.h,
        },
    })
end

---Move a window to a tag, removing all other ones.
---
---### Example
---```lua
----- With the focused window on tags 1, 2, 3, and 4...
---window.get_focused():move_to_tag("5")
----- ...will make the window only appear on tag 5.
---```
---@param name string The name of the tag.
function win:move_to_tag(name)
    SendMsg({
        MoveWindowToTag = {
            window_id = self.id,
            tag_id = name,
        },
    })
end

---Toggle the specified tag for this window.
---
---Note: toggling off all tags currently makes a window not response to layouting.
---
---### Example
---```lua
----- With the focused window only on tag 1...
---window.get_focused():toggle_tag("2")
----- ...will also make the window appear on tag 2.
---```
---@param name string The name of the tag.
function win:toggle_tag(name)
    SendMsg({
        ToggleTagOnWindow = {
            window_id = self.id,
            tag_id = name,
        },
    })
end

---Close this window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---### Example
---```lua
---window.get_focused():close() -- close the currently focused window
---```
function win:close()
    SendMsg({
        CloseWindow = {
            window_id = self.id,
        },
    })
end

---Toggle this window's floating status.
---
---### Example
---```lua
---window.get_focused():toggle_floating() -- toggles the focused window between tiled and floating
---```
function win:toggle_floating()
    SendMsg({
        ToggleFloating = {
            window_id = self.id,
        },
    })
end

---Get a window's size.
---
---### Example
---```lua
----- With a 4K monitor, given a focused fullscreen window...
---local size = window.get_focused():size()
----- ...should have size equal to `{ w = 3840, h = 2160 }`.
---```
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
function win:size()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local size = response.RequestResponse.response.WindowProps.size
    if size == nil then
        return nil
    else
        return {
            w = size[1],
            h = size[2],
        }
    end
end

---Get this window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The top left of the sheet if you trim it down is (0, 0).
---The location of this window is relative to that point.
---
---### Example
---```lua
----- With two 1080p monitors side by side and set up as such,
----- if a window is fullscreen on the right one...
---local loc = that_window:loc()
----- ...should have loc equal to `{ x = 1920, y = 0 }`.
---```
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
function win:loc()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local loc = response.RequestResponse.response.WindowProps.loc
    if loc == nil then
        return nil
    else
        return {
            x = loc[1],
            y = loc[2],
        }
    end
end

---Get this window's class. This is usually the name of the application.
---
---### Example
---```lua
----- With Alacritty focused...
---print(window.get_focused():class())
----- ...should print "Alacritty".
---```
---@return string|nil class This window's class, or nil if it doesn't exist.
function win:class()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local class = response.RequestResponse.response.WindowProps.class
    return class
end

---Get this window's title.
---
---### Example
---```lua
----- With Alacritty focused...
---print(window.get_focused():title())
----- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---```
---@return string|nil title This window's title, or nil if it doesn't exist.
function win:title()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local title = response.RequestResponse.response.WindowProps.title
    return title
end

---Get this window's floating status.
---
---### Example
---```lua
----- With the focused window floating...
---print(window.get_focused():floating())
----- ...should print `true`.
---```
---@return boolean|nil floating `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.
function win:floating()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local floating = response.RequestResponse.response.WindowProps.floating
    return floating
end

---Get whether or not this window is focused.
---
---### Example
---```lua
---print(window.get_focused():focused()) -- should print `true`.
---```
---@return boolean|nil floating `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.
function win:focused()
    SendRequest({
        GetWindowProps = {
            window_id = self.id,
        },
    })

    local response = ReadMsg()
    local focused = response.RequestResponse.response.WindowProps.focused
    return focused
end

-------------------------------------------------------------------

---@class WindowGlobal
local window = {}

---Get all windows with the specified class (usually the name of the application).
---@param class string The class. For example, Alacritty's class is "Alacritty".
---@return Window[]
function window.get_by_class(class)
    SendRequest("GetWindows")

    local response = ReadMsg()

    local window_ids = response.RequestResponse.response.Windows.window_ids

    ---@type Window[]
    local windows = {}
    for _, window_id in pairs(window_ids) do
        local w = new_window({ id = window_id })
        if w:class() == class then
            table.insert(windows, w)
        end
    end

    return windows
end

---Get all windows with the specified title.
---@param title string The title.
---@return Window[]
function window.get_by_title(title)
    SendRequest("GetWindows")

    local response = ReadMsg()

    local window_ids = response.RequestResponse.response.Windows.window_ids

    ---@type Window[]
    local windows = {}
    for _, window_id in pairs(window_ids) do
        local w = new_window({ id = window_id })
        if w:title() == title then
            table.insert(windows, w)
        end
    end

    return windows
end

---Get the currently focused window.
---@return Window|nil
function window.get_focused()
    SendRequest("GetWindows")

    local response = ReadMsg()

    local window_ids = response.RequestResponse.response.Windows.window_ids

    for _, window_id in pairs(window_ids) do
        local w = new_window({ id = window_id })
        if w:focused() then
            return w
        end
    end

    return nil
end

---Get all windows.
---@return Window[]
function window.get_all()
    SendRequest("GetWindows")

    local window_ids = ReadMsg().RequestResponse.response.Windows.window_ids
    ---@type Window[]
    local windows = {}
    for _, window_id in pairs(window_ids) do
        table.insert(windows, new_window({ id = window_id }))
    end
    return windows
end

return window
