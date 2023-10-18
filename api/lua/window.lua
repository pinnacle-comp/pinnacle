-- SPDX-License-Identifier: GPL-3.0-or-later

---Window management.
---
---This module helps you deal with setting windows to fullscreen and maximized, setting their size,
---moving them between tags, and various other actions.
---@class Window
local window = {
    ---Window rules.
    rules = require("window_rules"),
}

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
---@field private _id integer The internal id of this window
local window_handle = {}

---@param window_id WindowId
---@return WindowHandle
local function create_window(window_id)
    ---@type WindowHandle
    local w = { _id = window_id }
    -- Copy functions over
    for k, v in pairs(window_handle) do
        w[k] = v
    end

    return w
end

---Get this window's unique id.
---
---***You will probably not need to use this.***
---@return WindowId
function window_handle:id()
    return self._id
end

---Set this window's size.
---
---See `Window.set_size` for examples.
---
---@param size { w: integer?, h: integer? }
---@see Window.set_size — The corresponding module function
function window_handle:set_size(size)
    window.set_size(self, size)
end

---Move this window to a tag, removing all other ones.
---
---See `Window.move_to_tag` for examples.
---
---@param t TagConstructor
---@see Window.move_to_tag — The corresponding module function
function window_handle:move_to_tag(t)
    window.move_to_tag(self, t)
end

---Toggle the specified tag for this window.
---
---Note: toggling off all tags currently makes a window not respond to layouting.
---
---See `Window.toggle_tag` for examples.
---@param t TagConstructor
---@see Window.toggle_tag — The corresponding module function
function window_handle:toggle_tag(t)
    window.toggle_tag(self, t)
end

---Close this window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---See `Window.close` for examples.
---@see Window.close — The corresponding module function
function window_handle:close()
    window.close(self)
end

---Get this window's size.
---
---See `Window.size` for examples.
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see Window.size — The corresponding module function
function window_handle:size()
    return window.size(self)
end

---Get this window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The location of this window is relative inside the sheet.
---
---If you don't set the location of your monitors, they will start at (0, 0)
---and extend rightward with their tops aligned.
---
---See `Window.loc` for examples.
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
---@see Window.loc — The corresponding module function
function window_handle:loc()
    return window.loc(self)
end

---Get this window's class. This is usually the name of the application.
---
---See `Window.class` for examples.
---@return string|nil class This window's class, or nil if it doesn't exist.
---@see Window.class — The corresponding module function
function window_handle:class()
    return window.class(self)
end

---Get this window's title.
---
---See `Window.title` for examples.
---@return string|nil title This window's title, or nil if it doesn't exist.
---@see Window.title — The corresponding module function
function window_handle:title()
    return window.title(self)
end

---Get this window's floating status.
---@return boolean|nil
---@see Window.floating — The corresponding module function
function window_handle:floating()
    return window.floating(self)
end

---Get this window's fullscreen status.
---@return boolean|nil
---@see Window.fullscreen — The corresponding module function
function window_handle:fullscreen()
    return window.fullscreen(self)
end

---Get this window's maximized status.
---@return boolean|nil
---@see Window.maximized — The corresponding module function
function window_handle:maximized()
    return window.maximized(self)
end

---Toggle this window's floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
function window_handle:toggle_floating()
    window.toggle_floating(self)
end

---Toggle this window's fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
function window_handle:toggle_fullscreen()
    window.toggle_fullscreen(self)
end

---Toggle this window's maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
function window_handle:toggle_maximized()
    window.toggle_maximized(self)
end

---Get whether or not this window is focused.
---
---See `Window.focused` for examples.
---@return boolean|nil
---@see Window.focused — The corresponding module function
function window_handle:focused()
    return window.focused(self)
end

-------------------------------------------------------------------

---Get all windows with the specified class (usually the name of the application).
---@param class string The class. For example, Alacritty's class is "Alacritty".
---@return WindowHandle[]
function window.get_by_class(class)
    local windows = window.get_all()

    ---@type WindowHandle[]
    local windows_ret = {}
    for _, w in pairs(windows) do
        if w:class() == class then
            table.insert(windows_ret, w)
        end
    end

    return windows_ret
end

---Get all windows with the specified title.
---@param title string The title.
---@return WindowHandle[]
function window.get_by_title(title)
    local windows = window.get_all()

    ---@type WindowHandle[]
    local windows_ret = {}
    for _, w in pairs(windows) do
        if w:title() == title then
            table.insert(windows_ret, w)
        end
    end

    return windows_ret
end

---Get the currently focused window.
---@return WindowHandle|nil
function window.get_focused()
    -- TODO: get focused on output
    local windows = window.get_all()

    for _, w in pairs(windows) do
        if w:focused() then
            return w
        end
    end

    return nil
end

---Get all windows.
---@return WindowHandle[]
function window.get_all()
    local window_ids = Request("GetWindows").RequestResponse.response.Windows.window_ids

    ---@type WindowHandle[]
    local windows = {}

    for _, window_id in pairs(window_ids) do
        table.insert(windows, create_window(window_id))
    end

    return windows
end

---Toggle the tag with the given name and (optional) output for the specified window.
---
---@param w WindowHandle
---@param t TagConstructor
---@see WindowHandle.toggle_tag — The corresponding object method
function window.toggle_tag(w, t)
    local t = require("tag").get(t)

    if t then
        SendMsg({
            ToggleTagOnWindow = {
                window_id = w:id(),
                tag_id = t:id(),
            },
        })
    end
end

---Move the specified window to the tag with the given name and (optional) output.
---
---@param w WindowHandle
---@param t TagConstructor
---@see WindowHandle.move_to_tag — The corresponding object method
function window.move_to_tag(w, t)
    local t = require("tag").get(t)

    if t then
        SendMsg({
            MoveWindowToTag = {
                window_id = w:id(),
                tag_id = t:id(),
            },
        })
    end
end

---Toggle `win`'s floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
---@param win WindowHandle
function window.toggle_floating(win)
    SendMsg({
        ToggleFloating = {
            window_id = win:id(),
        },
    })
end

---Toggle `win`'s fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
---@param win WindowHandle
function window.toggle_fullscreen(win)
    SendMsg({
        ToggleFullscreen = {
            window_id = win:id(),
        },
    })
end

---Toggle `win`'s maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
---@param win WindowHandle
function window.toggle_maximized(win)
    SendMsg({
        ToggleMaximized = {
            window_id = win:id(),
        },
    })
end

---Set the specified window's size.
---
---### Examples
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    window.set_size(win, { w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---    window.set_size(win, { h = 300 })          -- keep the window's width but make it 300 pixels tall
---    window.set_size(win, {})                   -- do absolutely nothing useful
---end
---```
---@param win WindowHandle
---@param size { w: integer?, h: integer? }
---@see WindowHandle.set_size — The corresponding object method
function window.set_size(win, size)
    SendMsg({
        SetWindowSize = {
            window_id = win:id(),
            width = size.w,
            height = size.h,
        },
    })
end

---Close the specified window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---### Example
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    window.close(win) -- close the currently focused window
---end
---```
---@param win WindowHandle
---@see WindowHandle.close — The corresponding object method
function window.close(win)
    SendMsg({
        CloseWindow = {
            window_id = win:id(),
        },
    })
end

---Get the specified window's size.
---
---### Example
---```lua
--- -- With a 4K monitor, given a focused fullscreen window `win`...
---local size = window.size(win)
--- -- ...should have size equal to `{ w = 3840, h = 2160 }`.
---```
---@param win WindowHandle
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see WindowHandle.size — The corresponding object method
function window.size(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
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

---Get the specified window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The location of this window is relative inside the sheet.
---
---If you don't set the location of your monitors, they will start at (0, 0)
---and extend rightward with their tops aligned.
---
---### Example
---```lua
--- -- With two 1080p monitors side by side and set up as such,
--- -- if a window `win` is fullscreen on the right one...
---local loc = window.loc(win)
--- -- ...should have loc equal to `{ x = 1920, y = 0 }`.
---```
---@param win WindowHandle
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
---@see WindowHandle.loc — The corresponding object method
function window.loc(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
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

---Get the specified window's class. This is usually the name of the application.
---
---### Example
---```lua
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.class(win))
---end
--- -- ...should print "Alacritty".
---```
---@param win WindowHandle
---@return string|nil class This window's class, or nil if it doesn't exist.
---@see WindowHandle.class — The corresponding object method
function window.class(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local class = response.RequestResponse.response.WindowProps.class
    return class
end

---Get the specified window's title.
---
---### Example
---```lua
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.title(win))
---end
--- -- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---```
---@param win WindowHandle
---@return string|nil title This window's title, or nil if it doesn't exist.
---@see WindowHandle.title — The corresponding object method
function window.title(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local title = response.RequestResponse.response.WindowProps.title
    return title
end

---Get this window's floating status.
---@param win WindowHandle
---@return boolean|nil
---@see WindowHandle.floating — The corresponding object method
function window.floating(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local floating = response.RequestResponse.response.WindowProps.floating
    return floating
end

---Get this window's fullscreen status.
---@param win WindowHandle
---@return boolean|nil
---@see WindowHandle.fullscreen — The corresponding object method
function window.fullscreen(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom = response.RequestResponse.response.WindowProps.fullscreen_or_maximized
    return fom == "Fullscreen"
end

---Get this window's maximized status.
---@param win WindowHandle
---@return boolean|nil
---@see WindowHandle.maximized — The corresponding object method
function window.maximized(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom = response.RequestResponse.response.WindowProps.fullscreen_or_maximized
    return fom == "Maximized"
end

---Get whether or not this window is focused.
---
---### Example
---```lua
---local win = window.get_focused()
---if win ~= nil then
---    print(window.focused(win)) -- Should print `true`
---end
---```
---@param win WindowHandle
---@return boolean|nil
---@see WindowHandle.focused — The corresponding object method
function window.focused(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local focused = response.RequestResponse.response.WindowProps.focused
    return focused
end

---Begin a window move.
---
---This will start a window move grab with the provided button on the window the pointer
---is currently hovering over. Once `button` is let go, the move will end.
---@param button MouseButton The button you want to trigger the move.
function window.begin_move(button)
    SendMsg({
        WindowMoveGrab = {
            button = button,
        },
    })
end

---Begin a window resize.
---
---This will start a window resize grab with the provided button on the window the
---pointer is currently hovering over. Once `button` is let go, the resize will end.
---@param button MouseButton
function window.begin_resize(button)
    SendMsg({
        WindowResizeGrab = {
            button = button,
        },
    })
end

return window
