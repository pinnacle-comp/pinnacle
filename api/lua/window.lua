-- SPDX-License-Identifier: GPL-3.0-or-later

---@class WindowModule
local window_module = {}

---@class Window
---@field private _id integer The internal id of this window
local window = {}

---@param window_id WindowId
---@return Window
local function create_window(window_id)
    ---@type Window
    local w = { _id = window_id }
    -- Copy functions over
    for k, v in pairs(window) do
        w[k] = v
    end

    return w
end

---Get this window's unique id.
---
---***You will probably not need to use this.***
---@return WindowId
function window:id()
    return self._id
end

---Set this window's size.
---
---### Examples
---```lua
---window.get_focused():set_size({ w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---window.get_focused():set_size({ h = 300 })          -- keep the window's width but make it 300 pixels tall
---window.get_focused():set_size({})                   -- do absolutely nothing useful
---```
---@param size { w: integer?, h: integer? }
---@see WindowModule.set_size — The corresponding module function
function window:set_size(size)
    window_module.set_size(self, size)
end

---Move this window to a tag, removing all other ones.
---
---### Example
---```lua
----- With the focused window on tags 1, 2, 3, and 4...
---window.get_focused():move_to_tag("5")
----- ...will make the window only appear on tag 5.
---```
---@param name string
---@param output Output?
---@overload fun(self: self, t: Tag)
---@see WindowModule.move_to_tag — The corresponding module function
function window:move_to_tag(name, output)
    window_module.move_to_tag(self, name, output)
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
---@param name string
---@param output Output?
---@overload fun(self: self, t: Tag)
---@see WindowModule.toggle_tag — The corresponding module function
function window:toggle_tag(name, output)
    window_module.toggle_tag(self, name, output)
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
---@see WindowModule.close — The corresponding module function
function window:close()
    window_module.close(self)
end

---Get this window's size.
---
---### Example
---```lua
----- With a 4K monitor, given a focused fullscreen window...
---local size = window.get_focused():size()
----- ...should have size equal to `{ w = 3840, h = 2160 }`.
---```
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see WindowModule.size — The corresponding module function
function window:size()
    return window_module.size(self)
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
---@see WindowModule.loc — The corresponding module function
function window:loc()
    return window_module.loc(self)
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
---@see WindowModule.class — The corresponding module function
function window:class()
    return window_module.class(self)
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
---@see WindowModule.title — The corresponding module function
function window:title()
    return window_module.title(self)
end

---Get this window's floating status.
---@return boolean|nil
---@see WindowModule.floating — The corresponding module function
function window:floating()
    return window_module.floating(self)
end

---Get this window's fullscreen status.
---@return boolean|nil
---@see WindowModule.fullscreen — The corresponding module function
function window:fullscreen()
    return window_module.fullscreen(self)
end

---Get this window's maximized status.
---@return boolean|nil
---@see WindowModule.maximized — The corresponding module function
function window:maximized()
    return window_module.maximized(self)
end

---Toggle this window's floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
function window:toggle_floating()
    window_module.toggle_floating(self)
end

---Toggle this window's fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
function window:toggle_fullscreen()
    window_module.toggle_fullscreen(self)
end

---Toggle this window's maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
function window:toggle_maximized()
    window_module.toggle_maximized(self)
end

---Get whether or not this window is focused.
---
---### Example
---```lua
---print(window.get_focused():focused()) -- should print `true`.
---```
---@return boolean|nil
---@see WindowModule.focused — The corresponding module function
function window:focused()
    return window_module.focused(self)
end

-------------------------------------------------------------------

---Get all windows with the specified class (usually the name of the application).
---@param class string The class. For example, Alacritty's class is "Alacritty".
---@return Window[]
function window_module.get_by_class(class)
    local windows = window_module.get_all()

    ---@type Window[]
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
---@return Window[]
function window_module.get_by_title(title)
    local windows = window_module.get_all()

    ---@type Window[]
    local windows_ret = {}
    for _, w in pairs(windows) do
        if w:title() == title then
            table.insert(windows_ret, w)
        end
    end

    return windows_ret
end

---Get the currently focused window.
---@return Window|nil
function window_module.get_focused()
    local windows = window_module.get_all()

    for _, w in pairs(windows) do
        if w:focused() then
            return w
        end
    end

    return nil
end

---Get all windows.
---@return Window[]
function window_module.get_all()
    local window_ids =
        Request("GetWindows").RequestResponse.response.Windows.window_ids
    ---@type Window[]
    local windows = {}
    for _, window_id in pairs(window_ids) do
        table.insert(windows, create_window(window_id))
    end
    return windows
end

---Toggle the tag with the given name and (optional) output for the specified window.
---You can also provide a tag object instead of a name and output.
---@param w Window
---@param name string
---@param output Output?
---@overload fun(w: Window, t: Tag)
---@see Window.toggle_tag — The corresponding object method
function window_module.toggle_tag(w, name, output)
    if type(name) == "table" then
        SendMsg({
            ToggleTagOnWindow = {
                window_id = w:id(),
                tag_id = name--[[@as Tag]]:id(),
            },
        })
        return
    end

    local output = output or require("output").get_focused()

    if output == nil then
        return
    end

    local tags = require("tag").get_by_name(name)
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == output:name() then
            SendMsg({
                ToggleTagOnWindow = {
                    window_id = w:id(),
                    tag_id = t:id(),
                },
            })
            return
        end
    end
end

---Move the specified window to the tag with the given name and (optional) output.
---You can also provide a tag object instead of a name and output.
---@param w Window
---@param name string
---@param output Output?
---@overload fun(w: Window, t: Tag)
---@see Window.move_to_tag — The corresponding object method
function window_module.move_to_tag(w, name, output)
    if type(name) == "table" then
        SendMsg({
            MoveWindowToTag = {
                window_id = w:id(),
                tag_id = name--[[@as Tag]]:id(),
            },
        })
        return
    end

    local output = output or require("output").get_focused()

    if output == nil then
        return
    end

    local tags = require("tag").get_by_name(name)
    for _, t in pairs(tags) do
        if t:output() and t:output():name() == output:name() then
            SendMsg({
                MoveWindowToTag = {
                    window_id = w:id(),
                    tag_id = t:id(),
                },
            })
            return
        end
    end
end

---Toggle `win`'s floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
---@param win Window
function window_module.toggle_floating(win)
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
---@param win Window
function window_module.toggle_fullscreen(win)
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
---@param win Window
function window_module.toggle_maximized(win)
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
---@param win Window
---@param size { w: integer?, h: integer? }
---@see Window.set_size — The corresponding object method
function window_module.set_size(win, size)
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
---@param win Window
---@see Window.close — The corresponding object method
function window_module.close(win)
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
----- With a 4K monitor, given a focused fullscreen window `win`...
---local size = window.size(win)
----- ...should have size equal to `{ w = 3840, h = 2160 }`.
---```
---@param win Window
---@return { w: integer, h: integer }|nil size The size of the window, or nil if it doesn't exist.
---@see Window.size — The corresponding object method
function window_module.size(win)
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
---The top left of the sheet if you trim it down is (0, 0).
---The location of this window is relative to that point.
---
---### Example
---```lua
----- With two 1080p monitors side by side and set up as such,
----- if a window `win` is fullscreen on the right one...
---local loc = window.loc(win)
----- ...should have loc equal to `{ x = 1920, y = 0 }`.
---```
---@param win Window
---@return { x: integer, y: integer }|nil loc The location of the window, or nil if it's not on-screen or alive.
---@see Window.loc — The corresponding object method
function window_module.loc(win)
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
----- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.class(win))
---end
----- ...should print "Alacritty".
---```
---@param win Window
---@return string|nil class This window's class, or nil if it doesn't exist.
---@see Window.class — The corresponding object method
function window_module.class(win)
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
----- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.title(win))
---end
----- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---```
---@param win Window
---@return string|nil title This window's title, or nil if it doesn't exist.
---@see Window.title — The corresponding object method
function window_module.title(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local title = response.RequestResponse.response.WindowProps.title
    return title
end

---Get this window's floating status.
---@param win Window
---@return boolean|nil
---@see Window.floating — The corresponding object method
function window_module.floating(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local floating = response.RequestResponse.response.WindowProps.floating
    return floating
end

---Get this window's fullscreen status.
---@param win Window
---@return boolean|nil
---@see Window.fullscreen — The corresponding object method
function window_module.fullscreen(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom =
        response.RequestResponse.response.WindowProps.fullscreen_or_maximized
    return fom == "Fullscreen"
end

---Get this window's maximized status.
---@param win Window
---@return boolean|nil
---@see Window.maximized — The corresponding object method
function window_module.maximized(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local fom =
        response.RequestResponse.response.WindowProps.fullscreen_or_maximized
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
---@param win Window
---@return boolean|nil
---@see Window.focused — The corresponding object method
function window_module.focused(win)
    local response = Request({
        GetWindowProps = {
            window_id = win:id(),
        },
    })
    local focused = response.RequestResponse.response.WindowProps.focused
    return focused
end
return window_module
