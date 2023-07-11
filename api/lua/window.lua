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

---Get a window's size.
---@return { w: integer, h: integer }
function win:get_size()
    return self.size
end

-------------------------------------------------------------------

local window = {}

---Close a window.
---@param client_id integer? The id of the window you want closed, or nil to close the currently focused window, if any.
function window.close_window(client_id)
    SendMsg({
        CloseWindow = {
            client_id = client_id,
        },
    })
end

---Toggle a window's floating status.
---@param client_id integer? The id of the window you want to toggle, or nil to toggle the currently focused window, if any.
function window.toggle_floating(client_id)
    SendMsg({
        ToggleFloating = {
            client_id = client_id,
        },
    })
end

---TODO: This function is not implemented yet.
---
---Get a window by its app id (aka its X11 class).
---@param app_id string The window's app id. For example, Alacritty's app id is "Alacritty".
---@return Window window -- TODO: nil
function window.get_by_app_id(app_id)
    SendRequest({
        GetWindowByAppId = {
            app_id = app_id,
        },
    })

    local response = ReadMsg()

    local props = response.RequestResponse.response.Window.window

    ---@type Window
    local wind = {
        id = props.id,
        app_id = props.app_id or "",
        title = props.title or "",
        size = {
            w = props.size[1],
            h = props.size[2],
        },
        location = {
            x = props.location[1],
            y = props.location[2],
        },
        floating = props.floating,
    }

    return new_window(wind)
end

---TODO: This function is not implemented yet.
---
---Get a window by its title.
---@param title string The window's title.
---@return Window
function window.get_by_title(title)
    SendRequest({
        GetWindowByTitle = {
            title = title,
        },
    })

    local response = ReadMsg()

    local props = response.RequestResponse.response.Window.window

    ---@type Window
    local wind = {
        id = props.id,
        app_id = props.app_id or "",
        title = props.title or "",
        size = {
            w = props.size[1],
            h = props.size[2],
        },
        location = {
            x = props.location[1],
            y = props.location[2],
        },
        floating = props.floating,
    }

    return new_window(wind)
end

---Get the currently focused window.
---@return Window
function window.get_focused()
    SendRequest("GetWindowByFocus")

    local response = ReadMsg()

    local props = response.RequestResponse.response.Window.window

    ---@type Window
    local wind = {
        id = props.id,
        app_id = props.app_id or "",
        title = props.title or "",
        size = {
            w = props.size[1],
            h = props.size[2],
        },
        location = {
            x = props.location[1],
            y = props.location[2],
        },
        floating = props.floating,
    }

    return new_window(wind)
end

---Get all windows.
---@return Window[]
function window.get_all()
    SendRequest("GetAllWindows")

    -- INFO: these read synchronously so this should always work IF the server works correctly

    local window_props = ReadMsg().RequestResponse.response.GetAllWindows.windows
    ---@type Window[]
    local windows = {}
    for i, v in ipairs(window_props) do
        windows[i] = {
            id = v.id,
            app_id = v.app_id or "",
            title = v.title or "",
            size = {
                w = v.size[1],
                h = v.size[2],
            },
            location = {
                x = v.location[1],
                y = v.location[2],
            },
            floating = v.floating,
        }
    end
    return windows
end

return window
