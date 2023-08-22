-- SPDX-License-Identifier: GPL-3.0-or-later

---Window management
---@module WindowModule
local window_module = {}

---Get all windows with the specified class (usually the name of the application).
---@tparam string class The class. For example, Alacritty's class is "Alacritty".
---@treturn Window[]
function window_module.get_by_class(class) end

---Get all windows with the specified title.
---@tparam string title The title.
---@treturn Window[]
function window_module.get_by_title(title) end

---Get the currently focused window.
---@treturn Window|nil
function window_module.get_focused() end

---Get all windows.
---@treturn Window[]
function window_module.get_all() end

---Toggle the tag with the given name and (optional) output for the specified window.
---@tparam Window w
---@tparam Tag|table|string t A tag object, string of a name, `{ [1]: string, [2]: (string|Output)? }`, or `{ name: string, output: (string|Output)? }`
---@see Window.toggle_tag
function window_module.toggle_tag(w, t) end

---Move the specified window to the tag with the given name and (optional) output.
---@tparam Window w
---@tparam Tag|table|string t A tag object, string of a name, `{ [1]: string, [2]: (string|Output)? }`, or `{ name: string, output: (string|Output)? }`
---@see Window.move_to_tag
function window_module.move_to_tag(w, t) end

---Set the specified window's size.
---
---@usage
---local win = window.get_focused()
---if win ~= nil then
---    window.set_size(win, { w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---    window.set_size(win, { h = 300 })          -- keep the window's width but make it 300 pixels tall
---    window.set_size(win, {})                   -- do absolutely nothing useful
---end
---@tparam Window win
---@tparam table size A table of the form { w: integer?, h: integer? }
---@see Window.set_size
function window_module.set_size(win, size) end

---Close the specified window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---@usage
---local win = window.get_focused()
---if win ~= nil then
---    window.close(win) -- close the currently focused window
---end
---@tparam Window win
---@see Window.close
function window_module.close(win) end

---Get the specified window's size.
---
---@usage
--- -- With a 4K monitor, given a focused fullscreen window `win`...
---local size = window.size(win)
--- -- ...should have size equal to `{ w = 3840, h = 2160 }`.
---@tparam Window win
---@treturn table|nil size The size of the window in the form { w: integer, h: integer }, or nil if it doesn't exist.
---@see Window.size
function window_module.size(win) end

---Get the specified window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The top left of the sheet if you trim it down is (0, 0).
---The location of this window is relative to that point.
---
---@usage
--- -- With two 1080p monitors side by side and set up as such,
--- -- if a window `win` is fullscreen on the right one...
---local loc = window.loc(win)
--- -- ...should have loc equal to `{ x = 1920, y = 0 }`.
---@tparam Window win
---@treturn table|nil loc The location of the window in the form { x: integer, y: integer }, or nil if it's not on-screen or alive.
---@see Window.loc
function window_module.loc(win) end

---Get the specified window's class. This is usually the name of the application.
---
---@usage
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.class(win))
---end
--- -- ...should print "Alacritty".
---@tparam Window win
---@treturn string|nil class This window's class, or nil if it doesn't exist.
---@see Window.class
function window_module.class(win) end

---Get the specified window's title.
---
---@usage
--- -- With Alacritty focused...
---local win = window.get_focused()
---if win ~= nil then
---    print(window.title(win))
---end
--- -- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---@tparam Window win
---@treturn string|nil title This window's title, or nil if it doesn't exist.
---@see Window.title
function window_module.title(win) end

---Toggle `win`'s floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
---@tparam Window win
function window_module.toggle_floating(win) end

---Toggle `win`'s fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
---@tparam Window win
function window_module.toggle_fullscreen(win) end

---Toggle `win`'s maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
---@tparam Window win
function window_module.toggle_maximized(win) end

---Get whether or not this window is focused.
---
---@usage
---local win = window.get_focused()
---if win ~= nil then
---    print(window.focused(win)) -- Should print `true`
---end
---@tparam Window win
---@treturn boolean|nil floating `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.
---@see Window.focused
function window_module.focused(win) end

---Get whether or not `win` is floating (true) or tiled (false).
---@treturn boolean|nil
function window_module.floating(win) end

---Get whether or not `win` is fullscreen.
---@treturn boolean|nil
function window_module.fullscreen(win) end

---Get whether or not `win` is maximized.
---@treturn boolean|nil
function window_module.maximized(win) end

--------------------------------------------------------

---The window object.
---@classmod Window
local window = {}

---Set this window's size.
---
---@usage
---window.get_focused():set_size({ w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
---window.get_focused():set_size({ h = 300 })          -- keep the window's width but make it 300 pixels tall
---window.get_focused():set_size({})                   -- do absolutely nothing useful
---@tparam table size A table of the form { `w`: `integer?`, `h`: `integer?` }
---@see WindowModule.set_size
function window:set_size(size) end

---Move this window to a tag, removing all other ones.
---
---@usage
--- -- With the focused window on tags 1, 2, 3, and 4...
---window.get_focused():move_to_tag("5")
--- -- ...will make the window only appear on tag 5.
---@tparam string name
---@tparam ?Output output
---@see WindowModule.move_to_tag
function window:move_to_tag(name, output) end

---Toggle the specified tag for this window.
---
---Note: toggling off all tags currently makes a window not response to layouting.
---
---@usage
--- -- With the focused window only on tag 1...
---window.get_focused():toggle_tag("2")
--- -- ...will also make the window appear on tag 2.
---@tparam string name
---@tparam ?Output output
---@see WindowModule.toggle_tag
function window:toggle_tag(name, output) end

---Close this window.
---
---This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
---This will trigger save prompts in applications like GIMP.
---
---@usage
---window.get_focused():close() -- close the currently focused window
---@see WindowModule.close
function window:close() end

---Get this window's size.
---
---@usage
--- -- With a 4K monitor, given a focused fullscreen window...
---local size = window.get_focused():size()
--- -- ...should have size equal to `{ w = 3840, h = 2160 }`.
---@treturn table|nil size The size of the window in the form { `w`: `integer`, `h`: `integer` }, or nil if it doesn't exist.
---@see WindowModule.size
function window:size() end

---Get this window's location in the global space.
---
---Think of your monitors as being laid out on a big sheet.
---The top left of the sheet if you trim it down is (0, 0).
---The location of this window is relative to that point.
---
---@usage
--- -- With two 1080p monitors side by side and set up as such,
--- -- if a window is fullscreen on the right one...
---local loc = that_window:loc()
--- -- ...should have loc equal to `{ x = 1920, y = 0 }`.
---@treturn table|nil loc The location of the window in the form { `x`: `integer`, `y`: `integer` }, or nil if it's not on-screen or alive.
---@see WindowModule.loc
function window:loc() end

---Get this window's class. This is usually the name of the application.
---
---@usage
--- -- With Alacritty focused...
---print(window.get_focused():class())
--- -- ...should print "Alacritty".
---@treturn string|nil class This window's class, or nil if it doesn't exist.
---@see WindowModule.class
function window:class() end

---Get this window's title.
---
---@usage
--- -- With Alacritty focused...
---print(window.get_focused():title())
--- -- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
---@treturn string|nil title This window's title, or nil if it doesn't exist.
---@see WindowModule.title
function window:title() end

---Toggle this window's floating status.
---
---When used on a floating window, this will change it to tiled, and vice versa.
---
---When used on a fullscreen or maximized window, this will still change its
---underlying floating/tiled status.
---@tparam Window win
function window:toggle_floating(win) end

---Toggle this window's fullscreen status.
---
---When used on a fullscreen window, this will change the window back to
---floating or tiled.
---
---When used on a non-fullscreen window, it becomes fullscreen.
---@tparam Window win
function window:toggle_fullscreen(win) end

---Toggle this window's maximized status.
---
---When used on a maximized window, this will change the window back to
---floating or tiled.
---
---When used on a non-maximized window, it becomes maximized.
---@tparam Window win
function window:toggle_maximized(win) end

---Get whether or not this window is focused.
---
---@usage
---print(window.get_focused():focused()) -- should print `true`.
---@treturn boolean|nil floating `true` if it's focused, `false` if it's tiled, or nil if it doesn't exist.
---@see WindowModule.focused
function window:focused() end

---Get whether or not this window is floating (true) or tiled (false).
---@treturn boolean|nil
function window:floating() end

---Get whether or not this window is fullscreen.
---@treturn boolean|nil
function window:fullscreen() end

---Get whether or not this window is maximized.
---@treturn boolean|nil
function window:maximized() end
