require("pinnacle").setup(function(Pinnacle)
    local Input = Pinnacle.input
    local Process = Pinnacle.process
    local Output = Pinnacle.output
    local Tag = Pinnacle.tag
    local Window = Pinnacle.window

    local key = Input.key

    ---@type Modifier
    local mod_key = "ctrl"

    local terminal = "alacritty"

    --------------------
    -- Mousebinds     --
    --------------------

    Input:mousebind({ mod_key }, "btn_left", "press", function()
        Window:begin_move("btn_left")
    end)

    Input:mousebind({ mod_key }, "btn_right", "press", function()
        Window:begin_resize("btn_right")
    end)

    --------------------
    -- Keybinds       --
    --------------------

    -- mod_key + alt + q = Quit Pinnacle
    Input:keybind({ mod_key, "alt" }, "q", function()
        Pinnacle:quit()
    end)

    -- mod_key + alt + c = Close window
    Input:keybind({ mod_key, "alt" }, "c", function()
        local focused = Window:get_focused()
        if focused then
            focused:close()
        end
    end)

    -- mod_key + alt + Return = Spawn `terminal`
    Input:keybind({ mod_key }, key.Return, function()
        Process:spawn(terminal)
    end)

    -- mod_key + alt + space = Toggle floating
    Input:keybind({ mod_key, "alt" }, key.space, function()
        local focused = Window:get_focused()
        if focused then
            focused:toggle_floating()
        end
    end)

    -- mod_key + f = Toggle fullscreen
    Input:keybind({ mod_key }, "f", function()
        local focused = Window:get_focused()
        if focused then
            focused:toggle_fullscreen()
        end
    end)

    -- mod_key + m = Toggle maximized
    Input:keybind({ mod_key }, "m", function()
        local focused = Window:get_focused()
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
    Output:connect_for_all(function(op)
        local tags = Tag:add(op, tag_names)
        tags[1]:set_active(true)
    end)

    -- Spawning must happen after you add tags, as Pinnacle currently doesn't render windows without tags.
    Process:spawn(terminal)

    -- Create a layout cycler to cycle layouts on an output.
    local layout_cycler = Tag:new_layout_cycler({
        "master_stack",
        "dwindle",
        "spiral",
        "corner_top_left",
        "corner_top_right",
        "corner_bottom_left",
        "corner_bottom_right",
    })

    -- mod_key + space = Cycle forward one layout on the focused output
    Input:keybind({ mod_key }, key.space, function()
        local focused_op = Output:get_focused()
        if focused_op then
            layout_cycler.next(focused_op)
        end
    end)

    -- mod_key + shift + space = Cycle backward one layout on the focused output
    Input:keybind({ mod_key, "shift" }, key.space, function()
        local focused_op = Output:get_focused()
        if focused_op then
            layout_cycler.prev(focused_op)
        end
    end)

    for _, tag_name in ipairs(tag_names) do
        -- nil-safety: tags are guaranteed to be on the outputs due to connect_for_all above

        -- mod_key + 1-5 = Switch to tags 1-5
        Input:keybind({ mod_key }, tag_name, function()
            Tag:get(tag_name):switch_to()
        end)

        -- mod_key + shift + 1-5 = Toggle tags 1-5
        Input:keybind({ mod_key, "shift" }, tag_name, function()
            Tag:get(tag_name):toggle_active()
        end)

        -- mod_key + alt + 1-5 = Move window to tags 1-5
        Input:keybind({ mod_key, "alt" }, tag_name, function()
            local focused = Window:get_focused()
            if focused then
                focused:move_to_tag(Tag:get(tag_name) --[[@as TagHandle]])
            end
        end)

        -- mod_key + shift + alt + 1-5 = Toggle tags 1-5 on window
        Input:keybind({ mod_key, "shift", "alt" }, tag_name, function()
            local focused = Window:get_focused()
            if focused then
                focused:toggle_tag(Tag:get(tag_name) --[[@as TagHandle]])
            end
        end)
    end
end)
