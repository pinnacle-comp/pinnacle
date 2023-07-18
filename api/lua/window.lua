-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@class Window
---@field private id integer The internal id of this window
---@field private app_id string? The equivalent of an X11 window's class
---@field private title string? The window's title
---@field private size { w: integer, h: integer } The size of the window
---@field private location { x: integer, y: integer } The location of the window
---@field private floating boolean Whether the window is floating or not (tiled)
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
---@param size { w: integer?, h: integer? }
function win:set_size(size)
    self.size = {
        w = size.w or self.size.w,
        h = size.h or self.size.h,
    }
    SendMsg({
        SetWindowSize = {
            window_id = self.id,
            size = { self.size.w, self.size.h },
        },
    })
end

---Move a window to a tag, removing all other ones.
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
function win:close()
    SendMsg({
        CloseWindow = {
            window_id = self.id,
        },
    })
end

---Toggle this window's floating status.
function win:toggle_floating()
    SendMsg({
        ToggleFloating = {
            window_id = self.id,
        },
    })
end

---Get a window's size.
---@return { w: integer, h: integer }
function win:get_size()
    return self.size
end

-------------------------------------------------------------------

local window = {}

---TODO: This function is not implemented yet.
---
---Get a window by its app id (aka its X11 class).
---@param app_id string The window's app id. For example, Alacritty's app id is "Alacritty".
---@return Window|nil
function window.get_by_app_id(app_id)
    SendRequest({
        GetWindowByAppId = {
            app_id = app_id,
        },
    })

    local response = ReadMsg()

    local window_id = response.RequestResponse.response.Window.window_id

    if window_id == nil then
        return nil
    end

    ---@type Window
    local wind = {
        id = window_id,
    }

    return new_window(wind)
end

---TODO: This function is not implemented yet.
---
---Get a window by its title.
---@param title string The window's title.
---@return Window|nil
function window.get_by_title(title)
    SendRequest({
        GetWindowByTitle = {
            title = title,
        },
    })

    local response = ReadMsg()

    local window_id = response.RequestResponse.response.Window.window_id

    if window_id == nil then
        return nil
    end

    ---@type Window
    local wind = {
        id = window_id,
    }

    return new_window(wind)
end

---Get the currently focused window.
---@return Window|nil
function window.get_focused()
    SendRequest("GetWindowByFocus")

    local response = ReadMsg()

    local window_id = response.RequestResponse.response.Window.window_id

    if window_id == nil then
        return nil
    end

    ---@type Window
    local wind = {
        id = window_id,
    }

    return new_window(wind)
end

---Get all windows.
---@return Window[]
function window.get_all()
    SendRequest("GetAllWindows")

    local window_ids = ReadMsg().RequestResponse.response.Windows.window_ids
    ---@type Window[]
    local windows = {}
    for i, window_id in ipairs(window_ids) do
        windows[i] = new_window({ id = window_id })
    end
    return windows
end

return window
