local Pinnacle = require("pinnacle")
local Input = require("pinnacle.input")
local Libinput = require("pinnacle.input.libinput")
local Process = require("pinnacle.process")
local Output = require("pinnacle.output")
local Tag = require("pinnacle.tag")
local Window = require("pinnacle.window")
local Layout = require("pinnacle.layout")
local Util = require("pinnacle.util")
-- `Snowcap` will be false when the Snowcap API isn't installed or Snowcap isn't running
local Snowcap = require("pinnacle.snowcap")

Pinnacle.setup(function()
    local key = Input.key

    ---@type pinnacle.input.Mod
    local mod_key = "super"
    -- Change the mod key to "alt" when running as a nested window
    if Pinnacle.backend() == "window" then
        mod_key = "alt"
    end

    local terminal = "alacritty"

    --------------------
    -- Mousebinds     --
    --------------------

    -- mod_key + left click drag = move a window
    Input.mousebind({ mod_key }, "btn_left", function()
        Window.begin_move("btn_left")
    end, {
        group = "Mouse",
        description = "Start an interactive window move",
    })

    -- mod_key + right click drag = resize a window
    Input.mousebind({ mod_key }, "btn_right", function()
        Window.begin_resize("btn_right")
    end, { group = "Mouse", description = "Start an interactive window resize" })

    --------------------
    -- Keybinds       --
    --------------------

    -- mod_key + s shows the keybind overlay
    if Snowcap then
        Input.keybind({ mod_key }, "s", function()
            Snowcap.integration.bind_overlay():show()
        end, {
            group = "Compositor",
            description = "Show the keybind overlay",
        })
    end

    if Snowcap then
        -- mod_key + shift + q = Quit Prompt
        Input.keybind({
            mods = { mod_key, "shift" },
            key = "q",
            on_press = function()
                Snowcap.integration.quit_prompt():show()
            end,
            group = "Compositor",
            description = "Show the quit prompt",
        })
        -- mod_key + ctrl + shift + q = Hardcoded quit
        Input.keybind({
            mods = { mod_key, "ctrl", "shift" },
            key = "q",
            quit = true,
            group = "Compositor",
            description = "Quit Pinnacle without prompt",
        })
    else
        -- mod_key + shift + q = Quit Pinnacle
        Input.keybind({
            mods = { mod_key, "shift" },
            key = "q",
            quit = true,
            group = "Compositor",
            description = "Quit Pinnacle",
        })
    end

    -- mod_key + ctrl + r = Reload config
    Input.keybind({
        mods = { mod_key, "ctrl" },
        key = "r",
        reload_config = true,
        group = "Compositor",
        description = "Reload the config",
    })

    -- mod_key + shift + c = Close window
    Input.keybind({ mod_key, "shift" }, "c", function()
        local focused = Window.get_focused()
        if focused then
            focused:close()
        end
    end, {
        group = "Window",
        description = "Close the focused window",
    })

    -- mod_key + Return = Spawn `terminal`
    Input.keybind({ mod_key }, key.Return, function()
        Process.spawn(terminal)
    end, {
        group = "Process",
        description = "Spawn a terminal",
    })

    -- mod_key + ctrl + space = Toggle floating
    Input.keybind({ mod_key, "ctrl" }, key.space, function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_floating()
            focused:raise()
        end
    end, {
        group = "Window",
        description = "Toggle floating on the focused window",
    })

    -- mod_key + f = Toggle fullscreen
    Input.keybind({ mod_key }, "f", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_fullscreen()
            focused:raise()
        end
    end, {
        group = "Window",
        description = "Toggle fullscreen on the focused window",
    })

    -- mod_key + m = Toggle maximized
    Input.keybind({ mod_key }, "m", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_maximized()
            focused:raise()
        end
    end, {
        group = "Window",
        description = "Toggle maximized on the focused window",
    })

    ----------------------
    -- Tags and Outputs --
    ----------------------

    local tag_names = { "1", "2", "3", "4", "5", "6", "7", "8", "9" }

    Output.for_each_output(function(output)
        local tags = Tag.add(output, tag_names)
        tags[1]:set_active(true)
    end)

    -- Tag keybinds
    for _, tag_name in ipairs(tag_names) do
        -- mod_key + 1-9 = Switch to tags 1-9
        Input.keybind({ mod_key }, tag_name, function()
            Tag.get(tag_name):switch_to()
        end, {
            group = "Tag",
            description = "Switch to tag " .. tag_name,
        })

        -- mod_key + ctrl + 1-9 = Toggle tags 1-9
        Input.keybind({ mod_key, "ctrl" }, tag_name, function()
            Tag.get(tag_name):toggle_active()
        end, {
            group = "Tag",
            description = "Toggle tag " .. tag_name,
        })

        -- mod_key + shift + 1-9 = Move window to tags 1-9
        Input.keybind({ mod_key, "shift" }, tag_name, function()
            local focused = Window.get_focused()
            if focused then
                focused:move_to_tag(Tag.get(tag_name) --[[@as pinnacle.tag.TagHandle]])
            end
        end, {
            group = "Tag",
            description = "Move the focused window to tag " .. tag_name,
        })

        -- mod_key + ctrl + shift + 1-9 = Toggle tags 1-9 on window
        Input.keybind({ mod_key, "ctrl", "shift" }, tag_name, function()
            local focused = Window.get_focused()
            if focused then
                focused:toggle_tag(Tag.get(tag_name) --[[@as pinnacle.tag.TagHandle]])
            end
        end, {
            group = "Tag",
            description = "Toggle tag " .. tag_name .. " on the focused window",
        })
    end

    --------------------
    -- Layouts        --
    --------------------

    -- Pinnacle supports a tree-based layout system built on layout nodes.
    --
    -- To determine the tree used to layout windows, Pinnacle requests your config for a tree data structure
    -- with nodes containing gaps, directions, etc. There are a few provided utilities for creating
    -- a layout, known as layout generators.
    --
    -- ### Layout generators ###
    -- A layout generator is a table that holds some state as well as
    -- the `layout` function, which takes in a window count and computes
    -- a tree of layout nodes that determines how windows are laid out.
    --
    -- There are currently six built-in layout generators, one of which delegates to other
    -- generators as shown below.

    -- Create a cycling layout generator. This provides methods to cycle
    -- between the provided layout generators below.
    local layout_cycler = Layout.builtin.cycle({
        -- `Layout.builtin` contains functions that create various layout generators.
        -- Each of these has settings that can be overridden by passing in a table with
        -- overriding options.
        Layout.builtin.master_stack(),
        Layout.builtin.master_stack({ master_side = "right" }),
        Layout.builtin.master_stack({ master_side = "top" }),
        Layout.builtin.master_stack({ master_side = "bottom" }),
        Layout.builtin.dwindle(),
        Layout.builtin.spiral(),
        Layout.builtin.corner(),
        Layout.builtin.corner({ corner_loc = "top_right" }),
        Layout.builtin.corner({ corner_loc = "bottom_left" }),
        Layout.builtin.corner({ corner_loc = "bottom_right" }),
        Layout.builtin.fair(),
        Layout.builtin.fair({ axis = "horizontal" }),
    })

    -- Use the cycling layout generator to manage layout requests.
    -- This returns an object that allows you to request layouts manually.
    local layout_requester = Layout.manage(function(args)
        local first_tag = args.tags[1]
        if not first_tag then
            return {
                children = {},
            }
        end
        layout_cycler.current_tag = first_tag
        return layout_cycler:layout(args.window_count)
    end)

    -- mod_key + space = Cycle forward one layout on the focused output
    --
    -- Yes, this is a bit verbose for my liking.
    -- You need to cycle the layout on the first active tag
    -- because that is the one that decides which layout is used.
    Input.keybind({ mod_key }, key.space, function()
        local focused_op = Output.get_focused()
        if focused_op then
            local tags = focused_op:tags() or {}
            local tag = nil

            ---@type (fun(): (boolean|nil))[]
            local tag_actives = {}
            for i, t in ipairs(tags) do
                tag_actives[i] = function()
                    return t:active()
                end
            end

            -- We are batching API calls here for better performance
            tag_actives = Util.batch(tag_actives)

            for i, active in ipairs(tag_actives) do
                if active then
                    tag = tags[i]
                    break
                end
            end

            if tag then
                layout_cycler:cycle_layout_forward(tag)
                layout_requester:request_layout(focused_op)
            end
        end
    end, {
        group = "Layout",
        description = "Cycle the layout forward on the first active tag",
    })

    -- mod_key + shift + space = Cycle backward one layout on the focused output
    Input.keybind({ mod_key, "shift" }, key.space, function()
        local focused_op = Output.get_focused()
        if focused_op then
            local tags = focused_op:tags() or {}
            local tag = nil

            ---@type (fun(): boolean)[]
            local tag_actives = {}
            for i, t in ipairs(tags) do
                tag_actives[i] = function()
                    return t:active()
                end
            end

            tag_actives = Util.batch(tag_actives)

            for i, active in ipairs(tag_actives) do
                if active then
                    tag = tags[i]
                    break
                end
            end

            if tag then
                layout_cycler:cycle_layout_backward(tag)
                layout_requester:request_layout(focused_op)
            end
        end
    end, {
        group = "Layout",
        description = "Cycle the layout backward on the first active tag",
    })

    Libinput.for_each_device(function(device)
        -- Enable natural scroll for touchpads
        if device:device_type() == "touchpad" then
            device:set_natural_scroll(true)
        end
    end)

    -- There are no server-side decorations yet, so request all clients use client-side decorations.
    Window.add_window_rule(function(window)
        window:set_decoration_mode("client_side")
    end)

    -- Enable sloppy focus
    Window.connect_signal({
        pointer_enter = function(window)
            window:set_focused(true)
        end,
    })

    -- Spawning should happen after you add tags, as Pinnacle currently doesn't render windows without tags.
    Process.spawn_once(terminal)
end)
