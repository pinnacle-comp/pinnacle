-- neovim users be like
require("pinnacle").setup(function(Pinnacle)
    local Input = Pinnacle.input
    local Process = Pinnacle.process
    local Output = Pinnacle.output
    local Tag = Pinnacle.tag
    local Window = Pinnacle.window
    local Layout = Pinnacle.layout
    local Util = Pinnacle.util

    local key = Input.key

    ---@type Modifier
    local mod_key = "ctrl"

    local terminal = "alacritty"

    --------------------
    -- Mousebinds     --
    --------------------

    Input.mousebind({ mod_key }, "btn_left", "press", function()
        Window.begin_move("btn_left")
    end)

    Input.mousebind({ mod_key }, "btn_right", "press", function()
        Window.begin_resize("btn_right")
    end)

    --------------------
    -- Keybinds       --
    --------------------

    -- mod_key + alt + q = Quit Pinnacle
    Input.keybind({ mod_key, "alt" }, "q", function()
        Pinnacle.quit()
    end)

    -- mod_key + alt + c = Close window
    Input.keybind({ mod_key, "alt" }, "c", function()
        local focused = Window.get_focused()
        if focused then
            focused:close()
        end
    end)

    -- mod_key + alt + Return = Spawn `terminal`
    Input.keybind({ mod_key }, key.Return, function()
        Process.spawn(terminal)
    end)

    -- mod_key + alt + space = Toggle floating
    Input.keybind({ mod_key, "alt" }, key.space, function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_floating()
            focused:raise()
        end
    end)

    -- mod_key + f = Toggle fullscreen
    Input.keybind({ mod_key }, "f", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_fullscreen()
            focused:raise()
        end
    end)

    -- mod_key + m = Toggle maximized
    Input.keybind({ mod_key }, "m", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_maximized()
            focused:raise()
        end
    end)

    ----------------------
    -- Tags and Outputs --
    ----------------------

    local tag_names = { "1", "2", "3", "4", "5" }

    -- Setup outputs.
    --
    -- `Output.setup` allows you to declare things like mode, scale, and tags for outputs.
    -- Here we give all outputs tags 1 through 5.
    --
    -- Note that output matching functions currently don't infer the type of the parameter,
    -- so you may need to add `---@param <param name> OutputHandle` above it.
    Output.setup({
        {
            function(_)
                return true
            end,
            tag_names = tag_names,
        },
    })

    -- If you want to declare output locations as well, you can use `Output.setup_locs`.
    -- This will additionally allow you to recalculate output locations on signals like
    -- output connect, disconnect, and resize.
    --
    -- Read the admittedly scuffed docs for more.

    -- Tag keybinds
    for _, tag_name in ipairs(tag_names) do
        -- nil-safety: tags are guaranteed to be on the outputs due to connect_for_all above

        -- mod_key + 1-5 = Switch to tags 1-5
        Input.keybind({ mod_key }, tag_name, function()
            Tag.get(tag_name):switch_to()
        end)

        -- mod_key + shift + 1-5 = Toggle tags 1-5
        Input.keybind({ mod_key, "shift" }, tag_name, function()
            Tag.get(tag_name):toggle_active()
        end)

        -- mod_key + alt + 1-5 = Move window to tags 1-5
        Input.keybind({ mod_key, "alt" }, tag_name, function()
            local focused = Window.get_focused()
            if focused then
                focused:move_to_tag(Tag.get(tag_name) --[[@as TagHandle]])
            end
        end)

        -- mod_key + shift + alt + 1-5 = Toggle tags 1-5 on window
        Input.keybind({ mod_key, "shift", "alt" }, tag_name, function()
            local focused = Window.get_focused()
            if focused then
                focused:toggle_tag(Tag.get(tag_name) --[[@as TagHandle]])
            end
        end)
    end

    --------------------
    -- Layouts        --
    --------------------

    -- Pinnacle does not manage layouts compositor-side.
    -- Instead, it delegates computation of layouts to your config,
    -- which provides an interface to calculate the size and location of
    -- windows that the compositor will use to position windows.
    --
    -- If you're familiar with River's layout generators, you'll understand the system here
    -- a bit better.
    --
    -- The Lua API provides two layout system abstractions:
    --     1. Layout managers, and
    --     2. Layout generators.
    --
    -- ### Layout Managers ###
    -- A layout manager is a table that contains a `get_active` function
    -- that returns some layout generator.
    -- A manager is meant to keep track of and choose various layout generators
    -- across your usage of the compositor.
    --
    -- ### Layout generators ###
    -- A layout generator is a table that holds some state as well as
    -- the `layout` function, which takes in layout arguments and computes
    -- an array of geometries that will determine the size and position
    -- of windows being laid out.
    --
    -- There is one built-in layout manager and five built-in layout generators,
    -- as shown below.
    --
    -- Additionally, this system is designed to be user-extensible;
    -- you are free to create your own layout managers and generators for
    -- maximum customizability! Docs for doing so are in the works, so sit tight.

    -- Create a cycling layout manager. This provides methods to cycle
    -- between the given layout generators below.
    local layout_manager = Layout.new_cycling_manager({
        -- `Layout.builtins` contains functions that create various layout generators.
        -- Each of these has settings that can be overridden by passing in a table with
        -- overriding options.
        Layout.builtins.master_stack(),
        Layout.builtins.master_stack({ master_side = "right" }),
        Layout.builtins.master_stack({ master_side = "top" }),
        Layout.builtins.master_stack({ master_side = "bottom" }),
        Layout.builtins.dwindle(),
        Layout.builtins.spiral(),
        Layout.builtins.corner(),
        Layout.builtins.corner({ corner_loc = "top_right" }),
        Layout.builtins.corner({ corner_loc = "bottom_left" }),
        Layout.builtins.corner({ corner_loc = "bottom_right" }),
        Layout.builtins.fair(),
        Layout.builtins.fair({ direction = "horizontal" }),
    })

    -- Set the cycling layout manager as the layout manager that will be used.
    -- This then allows you to call `Layout.request_layout` to manually layout windows.
    Layout.set_manager(layout_manager)

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
                layout_manager:cycle_layout_forward(tag)
                Layout.request_layout(focused_op)
            end
        end
    end)

    -- mod_key + shift + space = Cycle backward one layout on the focused output
    Input.keybind({ mod_key, "shift" }, key.space, function()
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

            tag_actives = Util.batch(tag_actives)

            for i, active in ipairs(tag_actives) do
                if active then
                    tag = tags[i]
                    break
                end
            end

            if tag then
                layout_manager:cycle_layout_backward(tag)
                Layout.request_layout(focused_op)
            end
        end
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
