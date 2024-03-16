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
        end
    end)

    -- mod_key + f = Toggle fullscreen
    Input.keybind({ mod_key }, "f", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_fullscreen()
        end
    end)

    -- mod_key + m = Toggle maximized
    Input.keybind({ mod_key }, "m", function()
        local focused = Window.get_focused()
        if focused then
            focused:toggle_maximized()
        end
    end)

    --------------------
    -- Tags           --
    --------------------

    local tag_names = { "1", "2", "3", "4", "5" }

    -- `connect_for_all` is useful for performing setup on every monitor you have.
    -- Here, we add tags with names 1-5 and set tag 1 as active.
    Output.connect_for_all(function(op)
        local tags = Tag.add(op, tag_names)
        tags[1]:set_active(true)
    end)

    --------------------
    -- Layouts        --
    --------------------

    -- TODO: convert layouts into objs, deep_copy doesn't work on fns

    local master_stack_right = Util.deep_copy(Layout.builtins.master_stack)
    master_stack_right.master_side = "right"
    local master_stack_top = Util.deep_copy(Layout.builtins.master_stack)
    master_stack_top.master_side = "top"
    local master_stack_bottom = Util.deep_copy(Layout.builtins.master_stack)
    master_stack_bottom.master_side = "bottom"

    local layout_manager = Layout.new_cycling_manager({
        Layout.builtins.master_stack,
        master_stack_right,
        master_stack_top,
        master_stack_bottom,
        Layout.builtins.dwindle,
        Layout.builtins.spiral,
        Layout.builtins.corner,
        Layout.builtins.fair,
    })

    Layout.set_manager(layout_manager)

    -- mod_key + space = Cycle forward one layout on the focused output
    Input.keybind({ mod_key }, key.space, function()
        local focused_op = Output.get_focused()
        if focused_op then
            local tags = focused_op:tags()
            local tag = nil
            for _, t in ipairs(tags or {}) do
                if t:active() then
                    tag = t
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
            local tags = focused_op:tags()
            local tag = nil
            for _, t in ipairs(tags or {}) do
                if t:active() then
                    tag = t
                    break
                end
            end
            if tag then
                layout_manager:cycle_layout_backward(tag)
                Layout.request_layout(focused_op)
            end
        end
    end)

    -- Spawning must happen after you add tags, as Pinnacle currently doesn't render windows without tags.
    Process.spawn_once(terminal)

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

    -- Enable sloppy focus
    Window.connect_signal({
        pointer_enter = function(window)
            window:set_focused(true)
        end,
    })
end)
