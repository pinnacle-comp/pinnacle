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

    Input:set_mousebind({ mod_key }, "left", "press", function()
        Window:begin_move("left")
    end)

    Input:set_mousebind({ mod_key }, "right", "press", function()
        Window:begin_resize("right")
    end)

    ------

    Input:set_keybind({ mod_key, "alt" }, "q", function()
        print("GOT QUIT")
        Pinnacle:quit()
    end)

    Input:set_keybind({ mod_key, "alt" }, "c", function()
        local focused = Window:get_focused()
        if focused then
            focused:close()
        end
    end)

    Input:set_keybind({ mod_key }, key.Return, function()
        Process:spawn(terminal)
    end)

    Input:set_keybind({ mod_key, "alt" }, key.space, function()
        local focused = Window:get_focused()
        if focused then
            focused:toggle_floating()
        end
    end)

    Input:set_keybind({ mod_key }, "f", function()
        local focused = Window:get_focused()
        if focused then
            focused:toggle_fullscreen()
        end
    end)

    Input:set_keybind({ mod_key }, "m", function()
        local focused = Window:get_focused()
        if focused then
            focused:toggle_maximized()
        end
    end)

    local tag_names = { "1", "2", "3", "4", "5" }

    Output:connect_for_all(function(op)
        local tags = Tag:add(op, tag_names)
        tags[1]:set_active(true)
    end)

    Process:spawn_once("foot")

    local layout_cycler = Tag:new_layout_cycler({
        "master_stack",
        "dwindle",
        "spiral",
        "corner_top_left",
        "corner_top_right",
        "corner_bottom_left",
        "corner_bottom_right",
    })

    Input:set_keybind({ mod_key }, key.space, function()
        local focused_op = Output:get_focused()
        if focused_op then
            layout_cycler.next(focused_op)
        end
    end)

    Input:set_keybind({ mod_key, "shift" }, key.space, function()
        local focused_op = Output:get_focused()
        if focused_op then
            layout_cycler.prev(focused_op)
        end
    end)

    for _, tag_name in ipairs(tag_names) do
        -- nil-safety: tags are guaranteed to be on the outputs due to connect_for_all above
        Input:set_keybind({ mod_key }, tag_name, function()
            Tag:get(tag_name):switch_to()
        end)
        Input:set_keybind({ mod_key, "shift" }, tag_name, function()
            Tag:get(tag_name):toggle_active()
        end)
        Input:set_keybind({ mod_key, "alt" }, tag_name, function()
            local focused = Window:get_focused()
            if focused then
                focused:move_to_tag(Tag:get(tag_name) --[[@as TagHandle]])
            end
        end)
        Input:set_keybind({ mod_key, "shift", "alt" }, tag_name, function()
            local focused = Window:get_focused()
            if focused then
                focused:toggle_tag(Tag:get(tag_name) --[[@as TagHandle]])
            end
        end)
    end
end)
