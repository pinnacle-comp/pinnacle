This is absolutely not automatible and I think I'll need to use LDoc, which means I now have the privilege of duplicating documentation. Yay!

# InputModule

## keybind


```lua
function InputModule.keybind(modifiers: ("Alt"|"Ctrl"|"Shift"|"Super")[], key: Keys, action: fun())
```

Set a keybind. If called with an already existing keybind, it gets replaced.

### Example

```lua
-- Set `Super + Return` to open Alacritty
input.keybind({ "Super" }, input.keys.Return, function()
    process.spawn("Alacritty")
end)
```

@*param* `key` — The key for the keybind.

@*param* `modifiers` — Which modifiers need to be pressed for the keybind to trigger.

@*param* `action` — What to do.



---

# Layout

```lua
Layout:
    | "MasterStack" -- One master window on the left with all other windows stacked to the right.
    | "Dwindle" -- Windows split in half towards the bottom right corner.
    | "Spiral" -- Windows split in half in a spiral.
    | "CornerTopLeft" -- One main corner window in the top left with a column of windows on the right and a row on the bottom.
    | "CornerTopRight" -- One main corner window in the top right with a column of windows on the left and a row on the bottom.
    | "CornerBottomLeft" -- One main corner window in the bottom left with a column of windows on the right and a row on the top.
    | "CornerBottomRight" -- One main corner window in the bottom right with a column of windows on the left and a row on the top.
```


```lua
"CornerBottomLeft"|"CornerBottomRight"|"CornerTopLeft"|"CornerTopRight"|"Dwindle"...(+2)
```


---

# Modifier

```lua
Modifier:
    | "Alt" -- The "Alt" key
    | "Ctrl" -- The "Control" key
    | "Shift" -- The "Shift" key
    | "Super" -- The "Super" key, aka "Meta", "Mod4" in X11, the Windows key, etc.
```


```lua
"Alt"|"Ctrl"|"Shift"|"Super"
```


---

# Output

## _name


```lua
string
```

The name of this output (or rather, of its connector).

## add_tags


```lua
(method) Output:add_tags(...string)
```

Add tags to this output.

@*param* `...` — The names of the tags you want to add. You can also pass in a table.

See: OutputModule.add_tags — The corresponding module function

## focused


```lua
(method) Output:focused()
  -> boolean|nil
```

Get whether or not this output is focused. This is currently defined as having the cursor on it.

See: OutputModule.focused — The corresponding module function

## loc


```lua
(method) Output:loc()
  -> { x: integer, y: integer }|nil
```

Get this output's location in the global space, in pixels.

See: OutputModule.loc — The corresponding module function

## make


```lua
(method) Output:make()
  -> string|nil
```

Get this output's make.

See: OutputModule.make — The corresponding module function

## model


```lua
(method) Output:model()
  -> string|nil
```

Get this output's model.

See: OutputModule.model — The corresponding module function

## name


```lua
(method) Output:name()
  -> string
```

Get this output's name. This is something like "eDP-1" or "HDMI-A-0".

## physical_size


```lua
(method) Output:physical_size()
  -> { w: integer, h: integer }|nil
```

Get this output's physical size in millimeters.

See: OutputModule.physical_size — The corresponding module function

## refresh_rate


```lua
(method) Output:refresh_rate()
  -> integer|nil
```

Get this output's refresh rate in millihertz.
For example, 60Hz will be returned as 60000.

See: OutputModule.refresh_rate — The corresponding module function

## res


```lua
(method) Output:res()
  -> { w: integer, h: integer }|nil
```

Get this output's resolution in pixels.

See: OutputModule.res — The corresponding module function

## tags


```lua
(method) Output:tags()
  -> Tag[]
```

Get all tags on this output.

See: OutputModule.tags — The corresponding module function


---

# OutputModule

## add_tags


```lua
function OutputModule.add_tags(op: Output, ...string)
```

Add tags to the specified output.

@*param* `...` — The names of the tags you want to add. You can also pass in a table.

See:
  * [TagModule.add](file:///home/jason/projects/pinnacle/api/lua/tag.lua#105#9) — The called function
  * [Output.add_tags](file:///home/jason/projects/pinnacle/api/lua/output.lua#46#9) — The corresponding object method

## connect_for_all


```lua
function OutputModule.connect_for_all(func: fun(output: Output))
```

Connect a function to be run on all current and future outputs.

When called, `connect_for_all` will immediately run `func` with all currently connected outputs.
If a new output is connected, `func` will also be called with it.

Please note: this function will be run *after* Pinnacle processes your entire config.
For example, if you define tags in `func` but toggle them directly after `connect_for_all`, nothing will happen as the tags haven't been added yet.

@*param* `func` — The function that will be run.

## focused


```lua
function OutputModule.focused(op: Output)
  -> boolean|nil
```

Get whether or not the specified output is focused. This is currently defined as having the cursor on it.

See: Output.focused — The corresponding object method

## get_by_model


```lua
function OutputModule.get_by_model(model: string)
  -> outputs: Output[]
```

Note: This may or may not be what is reported by other monitor listing utilities. Pinnacle currently fails to pick up one of my monitors' models when it is correctly picked up by tools like wlr-randr. I'll fix this in the future.

Get outputs by their model.
This is something like "DELL E2416H" or whatever gibberish monitor manufacturers call their displays.

@*param* `model` — The model of the output(s).

@*return* `outputs` — All outputs with this model.

## get_by_name


```lua
function OutputModule.get_by_name(name: string)
  -> output: Output|nil
```

Get an output by its name.

"Name" in this sense does not mean its model or manufacturer;
rather, "name" is the name of the connector the output is connected to.
This should be something like "HDMI-A-0", "eDP-1", or similar.

### Example
```lua
local monitor = output.get_by_name("DP-1")
print(monitor.name) -- should print `DP-1`
```

@*param* `name` — The name of the output.

@*return* `output` — The output, or nil if none have the provided name.

## get_by_res


```lua
function OutputModule.get_by_res(width: integer, height: integer)
  -> outputs: Output[]
```

Get outputs by their resolution.

@*param* `width` — The width of the outputs, in pixels.

@*param* `height` — The height of the outputs, in pixels.

@*return* `outputs` — All outputs with this resolution.

## get_focused


```lua
function OutputModule.get_focused()
  -> output: Output|nil
```

Get the currently focused output. This is currently implemented as the one with the cursor on it.

This function may return nil, which means you may get a warning if you try to use it without checking for nil.
Usually this function will not be nil unless you unplug all monitors, so instead of checking,
you can ignore the warning by either forcing the type to be non-nil with an inline comment:
```lua
local op = output.get_focused() --[[@as Output]]
```
or by disabling nil check warnings for the line:
```lua
local op = output.get_focused()
---@diagnostic disable-next-line:need-check-nil
local tags_on_output = op:tags()
```
Type checking done by Lua LS isn't perfect.
Note that directly using the result of this function inline will *not* raise a warning, so be careful.
```lua
local tags = output.get_focused():tags() -- will NOT warn for nil
```

@*return* `output` — The output, or nil if none are focused.

## get_for_tag


```lua
function OutputModule.get_for_tag(tag: Tag)
  -> Output|nil
```

Get the output the specified tag is on.

See:
  * [TagModule.output](file:///home/jason/projects/pinnacle/api/lua/tag.lua#396#9) — A global method for fully qualified syntax (for you Rustaceans out there)
  * [Tag.output](file:///home/jason/projects/pinnacle/api/lua/tag.lua#62#9) — The corresponding object method

## loc


```lua
function OutputModule.loc(op: Output)
  -> { x: integer, y: integer }|nil
```

Get the specified output's location in the global space, in pixels.

See: Output.loc — The corresponding object method

## make


```lua
function OutputModule.make(op: Output)
  -> string|nil
```

Get the specified output's make.

See: Output.make — The corresponding object method

## model


```lua
function OutputModule.model(op: Output)
  -> string|nil
```

Get the specified output's model.

See: Output.model — The corresponding object method

## physical_size


```lua
function OutputModule.physical_size(op: Output)
  -> { w: integer, h: integer }|nil
```

Get the specified output's physical size in millimeters.

See: Output.physical_size — The corresponding object method

## refresh_rate


```lua
function OutputModule.refresh_rate(op: Output)
  -> integer|nil
```

Get the specified output's refresh rate in millihertz.
For example, 60Hz will be returned as 60000.

See: Output.refresh_rate — The corresponding object method

## res


```lua
function OutputModule.res(op: Output)
  -> { w: integer, h: integer }|nil
```

Get the specified output's resolution in pixels.

See: Output.res — The corresponding object method

## tags


```lua
function OutputModule.tags(op: Output)
  -> Tag[]
```

Get the specified output's tags.

See:
  * [TagModule.get_on_output](file:///home/jason/projects/pinnacle/api/lua/tag.lua#282#9) — The called function
  * [Output.tags](file:///home/jason/projects/pinnacle/api/lua/output.lua#38#9) — The corresponding object method


---

# OutputName


```lua
string
```


---

# Pinnacle

## quit


```lua
function Pinnacle.quit()
```

Quit Pinnacle.

## setup


```lua
function Pinnacle.setup(config_func: fun(pinnacle: Pinnacle))
```

Configure Pinnacle. You should put mostly eveything into the config_func to avoid invalid state.
The function takes one argument: the Pinnacle table, which is how you'll access all of the available config options.


---

# ProcessModule

## spawn


```lua
function ProcessModule.spawn(command: string|string[], callback?: fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil))
```

Spawn a process with an optional callback for its stdout, stderr, and exit information.

`callback` has the following parameters:
 - `stdout`: The process's stdout printed this line.
 - `stderr`: The process's stderr printed this line.
 - `exit_code`: The process exited with this code.
 - `exit_msg`: The process exited with this message.

@*param* `command` — The command as one whole string or a table of each of its arguments

@*param* `callback` — A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.

## spawn_once


```lua
function ProcessModule.spawn_once(command: string|string[], callback?: fun(stdout: string|nil, stderr: string|nil, exit_code: integer|nil, exit_msg: string|nil))
```

Spawn a process only if it isn't already running, with an optional callback for its stdout, stderr, and exit information.

`callback` has the following parameters:
 - `stdout`: The process's stdout printed this line.
 - `stderr`: The process's stderr printed this line.
 - `exit_code`: The process exited with this code.
 - `exit_msg`: The process exited with this message.

`spawn_once` checks for the process using `pgrep`. If your system doesn't have `pgrep`, this won't work properly.

@*param* `command` — The command as one whole string or a table of each of its arguments

@*param* `callback` — A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.


---

# Tag

## _id


```lua
integer
```

The internal id of this tag.

## active


```lua
(method) Tag:active()
  -> active: boolean|nil
```

Get this tag's active status.

@*return* `active` — `true` if the tag is active, `false` if not, and `nil` if the tag doesn't exist.

See: TagModule.active — The corresponding module function

## id


```lua
(method) Tag:id()
  -> integer
```

Get this tag's internal id.
***You probably won't need to use this.***

## name


```lua
(method) Tag:name()
  -> name: string|nil
```

Get this tag's name.

@*return* `name` — The name of this tag, or nil if it doesn't exist.

See: TagModule.name — The corresponding module function

## output


```lua
(method) Tag:output()
  -> output: Output|nil
```

Get this tag's output.

@*return* `output` — The output this tag is on, or nil if the tag doesn't exist.

See: TagModule.output — The corresponding module function

## set_layout


```lua
(method) Tag:set_layout(layout: "CornerBottomLeft"|"CornerBottomRight"|"CornerTopLeft"|"CornerTopRight"|"Dwindle"...(+2))
```

Set this tag's layout.

```lua
layout:
    | "MasterStack" -- One master window on the left with all other windows stacked to the right.
    | "Dwindle" -- Windows split in half towards the bottom right corner.
    | "Spiral" -- Windows split in half in a spiral.
    | "CornerTopLeft" -- One main corner window in the top left with a column of windows on the right and a row on the bottom.
    | "CornerTopRight" -- One main corner window in the top right with a column of windows on the left and a row on the bottom.
    | "CornerBottomLeft" -- One main corner window in the bottom left with a column of windows on the right and a row on the top.
    | "CornerBottomRight" -- One main corner window in the bottom right with a column of windows on the left and a row on the top.
```

See: TagModule.set_layout — The corresponding module function

## switch_to


```lua
(method) Tag:switch_to()
```

Switch to this tag.

See: TagModule.switch_to — The corresponding module function

## toggle


```lua
(method) Tag:toggle()
```

Toggle this tag.

See: TagModule.toggle — The corresponding module function


---

# TagId


```lua
integer
```


---

# TagModule

## active


```lua
function TagModule.active(t: Tag)
  -> boolean|nil
```

Get whether or not the specified tag is active.

See: Tag.active — The corresponding object method

## add


```lua
function TagModule.add(output: Output, ...string)
```

Add tags to the specified output.

### Examples
```lua
local op = output.get_by_name("DP-1")
if op ~= nil then
    tag.add(op, "1", "2", "3", "4", "5") -- Add tags with names 1-5
end
```
You can also pass in a table.
```lua
local tags = {"Terminal", "Browser", "Code", "Potato", "Email"}
tag.add(op, tags) -- Add tags with those names
```

@*param* `output` — The output you want these tags to be added to.

@*param* `...` — The names of the new tags you want to add.

See: Output.add_tags — The corresponding object method

## get_all


```lua
function TagModule.get_all()
  -> Tag[]
```

Get all tags across all outputs.

### Example
```lua
-- With two monitors with the same tags: "1", "2", "3", "4", and "5"...
local tags = tag.get_all()
-- ...`tags` should have 10 tags, with 5 pairs of those names across both outputs.
```

## get_by_name


```lua
function TagModule.get_by_name(name: string)
  -> Tag[]
```

Get all tags with this name across all outputs.

### Example
```lua
-- Given one monitor with the tags "OBS", "OBS", "VSCode", and "Spotify"...
local tags = tag.get_by_name("OBS")
-- ...will have 2 tags in `tags`, while...
local no_tags = tag.get_by_name("Firefox")
-- ...will have `no_tags` be empty.
```

@*param* `name` — The name of the tag(s) you want.

## get_on_output


```lua
function TagModule.get_on_output(output: Output)
  -> Tag[]
```

Get all tags on the specified output.

### Example
```lua
local op = output.get_focused()
if op ~= nil then
    local tags = tag.get_on_output(op) -- All tags on the focused output
end
```

See: Output.tags — The corresponding object method

## name


```lua
function TagModule.name(t: Tag)
  -> string|nil
```

Get the specified tag's name.

### Example
```lua
-- Assuming the tag `Terminal` exists...
print(tag.name(tag.get_by_name("Terminal")[1]))
-- ...should print `Terminal`.
```

See: Tag.name — The corresponding object method

## output


```lua
function TagModule.output(t: Tag)
  -> Output|nil
```

Get the output the specified tag is on.

See:
  * [OutputModule.get_for_tag](file:///home/jason/projects/pinnacle/api/lua/output.lua#232#9) — The called function
  * [Tag.output](file:///home/jason/projects/pinnacle/api/lua/tag.lua#62#9) — The corresponding object method

## set_layout


```lua
function TagModule.set_layout(name: string, layout: "CornerBottomLeft"|"CornerBottomRight"|"CornerTopLeft"|"CornerTopRight"|"Dwindle"...(+2), output?: Output)
```

Set a layout for the tag on the specified output. If no output is provided, set it for the tag on the currently focused one.
Alternatively, provide a tag object instead of a name and output.

### Examples
```lua
-- Set tag `1` on `DP-1` to the `Dwindle` layout
tag.set_layout("1", "Dwindle", output.get_by_name("DP-1"))

-- Do the same as above. Note: if you have more than one tag named `1` then this picks the first one.
local t = tag.get_by_name("1")[1]
tag.set_layout(t, "Dwindle")
```

@*param* `name` — The name of the tag.

@*param* `layout` — The layout.

@*param* `output` — The output.

---

```lua
layout:
    | "MasterStack" -- One master window on the left with all other windows stacked to the right.
    | "Dwindle" -- Windows split in half towards the bottom right corner.
    | "Spiral" -- Windows split in half in a spiral.
    | "CornerTopLeft" -- One main corner window in the top left with a column of windows on the right and a row on the bottom.
    | "CornerTopRight" -- One main corner window in the top right with a column of windows on the left and a row on the bottom.
    | "CornerBottomLeft" -- One main corner window in the bottom left with a column of windows on the right and a row on the top.
    | "CornerBottomRight" -- One main corner window in the bottom right with a column of windows on the left and a row on the top.
```

See: Tag.set_layout — The corresponding object method

## switch_to


```lua
function TagModule.switch_to(name: string, output?: Output)
```

Switch to a tag on the specified output, deactivating any other active tags on it.
If `output` is not specified, this uses the currently focused output instead.
Alternatively, provide a tag object instead of a name and output.

This is used to replicate what a traditional workspace is on some other Wayland compositors.

### Examples
```lua
-- Switches to and displays *only* windows on tag `3` on the focused output.
tag.switch_to("3")

local
```

@*param* `name` — The name of the tag.

@*param* `output` — The output.

See: Tag.switch_to — The corresponding object method

## toggle


```lua
function TagModule.toggle(name: string, output?: Output)
```

Toggle a tag on the specified output. If `output` isn't specified, toggle it on the currently focused output instead.

### Example

```lua
-- Assuming all tags are toggled off...
local op = output.get_by_name("DP-1")
tag.toggle("1", op)
tag.toggle("2", op)
-- will cause windows on both tags 1 and 2 to be displayed at the same time.
```

@*param* `name` — The name of the tag.

@*param* `output` — The output.

See: Tag.toggle — The corresponding object method


---

# Window

## _id


```lua
integer
```

The internal id of this window

## class


```lua
(method) Window:class()
  -> class: string|nil
```

Get this window's class. This is usually the name of the application.

### Example
```lua
-- With Alacritty focused...
print(window.get_focused():class())
-- ...should print "Alacritty".
```

@*return* `class` — This window's class, or nil if it doesn't exist.

See: WindowModule.class — The corresponding module function

## close


```lua
(method) Window:close()
```

Close this window.

This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
This will trigger save prompts in applications like GIMP.

### Example
```lua
window.get_focused():close() -- close the currently focused window
```

See: WindowModule.close — The corresponding module function

## floating


```lua
(method) Window:floating()
  -> floating: boolean|nil
```

Get this window's floating status.

### Example
```lua
-- With the focused window floating...
print(window.get_focused():floating())
-- ...should print `true`.
```

@*return* `floating` — `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.

See: WindowModule.floating — The corresponding module function

## focused


```lua
(method) Window:focused()
  -> floating: boolean|nil
```

Get whether or not this window is focused.

### Example
```lua
print(window.get_focused():focused()) -- should print `true`.
```

@*return* `floating` — `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.

See: WindowModule.focused — The corresponding module function

## id


```lua
(method) Window:id()
  -> integer
```

Get this window's unique id.

***You will probably not need to use this.***

## loc


```lua
(method) Window:loc()
  -> loc: { x: integer, y: integer }|nil
```

Get this window's location in the global space.

Think of your monitors as being laid out on a big sheet.
The top left of the sheet if you trim it down is (0, 0).
The location of this window is relative to that point.

### Example
```lua
-- With two 1080p monitors side by side and set up as such,
-- if a window is fullscreen on the right one...
local loc = that_window:loc()
-- ...should have loc equal to `{ x = 1920, y = 0 }`.
```

@*return* `loc` — The location of the window, or nil if it's not on-screen or alive.

See: WindowModule.loc — The corresponding module function

## move_to_tag


```lua
(method) Window:move_to_tag(name: string, output?: Output)
```

Move this window to a tag, removing all other ones.

### Example
```lua
-- With the focused window on tags 1, 2, 3, and 4...
window.get_focused():move_to_tag("5")
-- ...will make the window only appear on tag 5.
```

See: WindowModule.move_to_tag — The corresponding module function

## set_size


```lua
(method) Window:set_size(size: { w: integer?, h: integer? })
```

Set this window's size.

### Examples
```lua
window.get_focused():set_size({ w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
window.get_focused():set_size({ h = 300 })          -- keep the window's width but make it 300 pixels tall
window.get_focused():set_size({})                   -- do absolutely nothing useful
```

See: WindowModule.set_size — The corresponding module function

## size


```lua
(method) Window:size()
  -> size: { w: integer, h: integer }|nil
```

Get this window's size.

### Example
```lua
-- With a 4K monitor, given a focused fullscreen window...
local size = window.get_focused():size()
-- ...should have size equal to `{ w = 3840, h = 2160 }`.
```

@*return* `size` — The size of the window, or nil if it doesn't exist.

See: WindowModule.size — The corresponding module function

## title


```lua
(method) Window:title()
  -> title: string|nil
```

Get this window's title.

### Example
```lua
-- With Alacritty focused...
print(window.get_focused():title())
-- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
```

@*return* `title` — This window's title, or nil if it doesn't exist.

See: WindowModule.title — The corresponding module function

## toggle_floating


```lua
(method) Window:toggle_floating()
```

Toggle this window's floating status.

### Example
```lua
window.get_focused():toggle_floating() -- toggles the focused window between tiled and floating
```

See: WindowModule.toggle_floating — The corresponding module function

## toggle_tag


```lua
(method) Window:toggle_tag(name: string, output?: Output)
```

Toggle the specified tag for this window.

Note: toggling off all tags currently makes a window not response to layouting.

### Example
```lua
-- With the focused window only on tag 1...
window.get_focused():toggle_tag("2")
-- ...will also make the window appear on tag 2.
```

See: WindowModule.toggle_tag — The corresponding module function


---

# WindowId


```lua
integer
```


---

# WindowModule

## class


```lua
function WindowModule.class(win: Window)
  -> class: string|nil
```

Get the specified window's class. This is usually the name of the application.

### Example
```lua
-- With Alacritty focused...
local win = window.get_focused()
if win ~= nil then
    print(window.class(win))
end
-- ...should print "Alacritty".
```

@*return* `class` — This window's class, or nil if it doesn't exist.

See: Window.class — The corresponding object method

## close


```lua
function WindowModule.close(win: Window)
```

Close the specified window.

This only sends a close *event* to the window and is the same as just clicking the X button in the titlebar.
This will trigger save prompts in applications like GIMP.

### Example
```lua
local win = window.get_focused()
if win ~= nil then
    window.close(win) -- close the currently focused window
end
```

See: Window.close — The corresponding object method

## floating


```lua
function WindowModule.floating(win: Window)
  -> floating: boolean|nil
```

Get this window's floating status.

### Example
```lua
-- With the focused window floating...
local win = window.get_focused()
if win ~= nil then
    print(window.floating(win))
end
-- ...should print `true`.
```

@*return* `floating` — `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.

See: Window.floating — The corresponding object method

## focused


```lua
function WindowModule.focused(win: Window)
  -> floating: boolean|nil
```

Get whether or not this window is focused.

### Example
```lua
local win = window.get_focused()
if win ~= nil then
    print(window.focused(win)) -- Should print `true`
end
```

@*return* `floating` — `true` if it's floating, `false` if it's tiled, or nil if it doesn't exist.

See: Window.focused — The corresponding object method

## get_all


```lua
function WindowModule.get_all()
  -> Window[]
```

Get all windows.

## get_by_class


```lua
function WindowModule.get_by_class(class: string)
  -> Window[]
```

Get all windows with the specified class (usually the name of the application).

@*param* `class` — The class. For example, Alacritty's class is "Alacritty".

## get_by_title


```lua
function WindowModule.get_by_title(title: string)
  -> Window[]
```

Get all windows with the specified title.

@*param* `title` — The title.

## get_focused


```lua
function WindowModule.get_focused()
  -> Window|nil
```

Get the currently focused window.

## loc


```lua
function WindowModule.loc(win: Window)
  -> loc: { x: integer, y: integer }|nil
```

Get the specified window's location in the global space.

Think of your monitors as being laid out on a big sheet.
The top left of the sheet if you trim it down is (0, 0).
The location of this window is relative to that point.

### Example
```lua
-- With two 1080p monitors side by side and set up as such,
-- if a window `win` is fullscreen on the right one...
local loc = window.loc(win)
-- ...should have loc equal to `{ x = 1920, y = 0 }`.
```

@*return* `loc` — The location of the window, or nil if it's not on-screen or alive.

See: Window.loc — The corresponding object method

## move_to_tag


```lua
function WindowModule.move_to_tag(w: Window, name: string, output?: Output)
```

Move the specified window to the tag with the given name and (optional) output.
You can also provide a tag object instead of a name and output.

See: Window.move_to_tag — The corresponding object method

## set_size


```lua
function WindowModule.set_size(win: Window, size: { w: integer?, h: integer? })
```

Set the specified window's size.

### Examples
```lua
local win = window.get_focused()
if win ~= nil then
    window.set_size(win, { w = 500, h = 500 }) -- make the window square and 500 pixels wide/tall
    window.set_size(win, { h = 300 })          -- keep the window's width but make it 300 pixels tall
    window.set_size(win, {})                   -- do absolutely nothing useful
end
```

See: Window.set_size — The corresponding object method

## size


```lua
function WindowModule.size(win: Window)
  -> size: { w: integer, h: integer }|nil
```

Get the specified window's size.

### Example
```lua
-- With a 4K monitor, given a focused fullscreen window `win`...
local size = window.size(win)
-- ...should have size equal to `{ w = 3840, h = 2160 }`.
```

@*return* `size` — The size of the window, or nil if it doesn't exist.

See: Window.size — The corresponding object method

## title


```lua
function WindowModule.title(win: Window)
  -> title: string|nil
```

Get the specified window's title.

### Example
```lua
-- With Alacritty focused...
local win = window.get_focused()
if win ~= nil then
    print(window.title(win))
end
-- ...should print the directory Alacritty is in or what it's running (what's in its title bar).
```

@*return* `title` — This window's title, or nil if it doesn't exist.

See: Window.title — The corresponding object method

## toggle_floating


```lua
function WindowModule.toggle_floating(win: Window)
```

Toggle the specified window between tiled and floating.

See: Window.toggle_floating — The corresponding object method

## toggle_tag


```lua
function WindowModule.toggle_tag(w: Window, name: string, output?: Output)
```

Toggle the tag with the given name and (optional) output for the specified window.
You can also provide a tag object instead of a name and output.

See: Window.toggle_tag — The corresponding object method


---

