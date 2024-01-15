local inspect = require("inspect")

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

    Output:connect_for_all(function(output)
        local tags = Tag:add(output, "1", "2", "3", "4", "5")
        tags[1]:set_active(true)
    end)

    local output = Output:get_all()[1]

    Process:spawn("alacritty")

    Input:keybind({}, "q", function()
        print("Keybind: q")
    end)
    Input:keybind({ "shift" }, "q", function()
        -- Should not happen, overridden by shift Q
        print("Keybind: shift q")
    end)
    Input:keybind({}, "Q", function()
        print("Keybind: Q")
    end)
    Input:keybind({ "shift" }, "Q", function()
        print("Keybind: shift Q")
    end)
    Input:keybind({}, "@", function()
        -- Should not happen, can't get @ without shift
        print("Keybind: @")
    end)
    Input:keybind({ "ctrl" }, "@", function()
        --- Should not happen, same as above
        print("Keybind: ctrl @")
    end)

    Input:keybind({ "shift" }, "a", function()
        local win = Window:get_focused()
        if win then
            win:set_fullscreen(true)
        end
    end)
    Input:keybind({ "shift" }, "s", function()
        local win = Window:get_focused()
        if win then
            win:set_fullscreen(false)
        end
    end)
    Input:keybind({ "shift" }, "d", function()
        local win = Window:get_focused()
        if win then
            win:set_maximized(true)
        end
    end)
    Input:keybind({ "shift" }, "f", function()
        local win = Window:get_focused()
        if win then
            win:set_maximized(false)
        end
    end)
    Input:keybind({ "shift" }, "g", function()
        local win = Window:get_focused()
        if win then
            win:set_floating(true)
        end
    end)
    Input:keybind({ "shift" }, "h", function()
        local win = Window:get_focused()
        if win then
            win:set_floating(false)
        end
    end)
    Input:keybind({ "shift" }, "j", function()
        local win = Window:get_focused()
        if win then
            win:toggle_fullscreen()
        end
    end)
    Input:keybind({ "shift" }, "k", function()
        local win = Window:get_focused()
        if win then
            win:toggle_maximized()
        end
    end)
    Input:keybind({ "shift" }, "l", function()
        local win = Window:get_focused()
        if win then
            win:toggle_floating()
        end
    end)

    Input:keybind({ "shift" }, "z", function()
        local win = Window:get_focused()
        if win then
            win:set_geometry({ x = 100, y = 200, width = 500, height = 200 })
        end
    end)
    Input:keybind({ "ctrl" }, "z", function()
        local win = Window:get_focused()
        if win then
            win:set_geometry({ width = 500, height = 200 })
        end
    end)

    Input:keybind({ "ctrl" }, key.Return, function()
        Process:spawn("alacritty")
    end)
    Input:keybind({ "shift" }, "x", function()
        print(inspect(Output:get_focused():props()))
        print(inspect(Window:get_focused():props()))
        print(inspect(Tag:get("1"):props()))
    end)
end)
