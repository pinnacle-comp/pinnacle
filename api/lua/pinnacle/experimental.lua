---Experimental APIs.
---
---IMPORTANT: These are unstable and may change at any moment.
local experimental = {}

---@class pinnacle.experimental.InputGrab : snowcap.widget.Program
local InputGrab = {}

function InputGrab:update(_) end

function InputGrab:view()
    return require("snowcap").widget.row({
        children = {},
        width = require("snowcap").widget.length.Fixed(1.0),
        height = require("snowcap").widget.length.Fixed(1.0),
    })
end

---Input grabbing.
local input_grab = nil

if require("pinnacle.snowcap") then
    input_grab = {}

    ---Grabs keyboard input.
    ---
    ---All keyboard input will be redirected to this grabber (assuming another exclusive layer
    ---surface doesn't open). Keybinds will still work.
    ---
    ---Don't forget to add a way to close the grabber, or else input will be grabbed forever!
    ---
    ---#### Example
    ---```lua
    ---require("pinnacle.experimental").input_grab.grab_input(function(grabber, key, mods)
    ---    if key == require("pinnacle.input").key.e then
    ---        print("An `e` was pressed!")
    ---    end
    ---
    ---    if key == require("pinnacle.input").key.Escape then
    ---        grabber:close()
    ---    end
    ---end)
    ---```
    ---
    ---@param with_input fun(handle: snowcap.layer.LayerHandle, key: pinnacle.input.Key, mods: snowcap.input.Modifiers)
    function input_grab.grab_input(with_input)
        local grabber = require("snowcap").layer.new_widget({
            layer = require("snowcap").layer.zlayer.OVERLAY,
            exclusive_zone = "respect",
            keyboard_interactivity = require("snowcap").layer.keyboard_interactivity.EXCLUSIVE,
            program = InputGrab,
        })

        if not grabber then
            return
        end

        grabber:on_key_press(function(mods, key)
            with_input(grabber, key, mods)
        end)
    end
end

---Input grabbing.
experimental.input_grab = input_grab

return experimental
