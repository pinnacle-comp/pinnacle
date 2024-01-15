require("pinnacle").setup(function(Pinnacle)
    local Input = Pinnacle.input
    local Process = Pinnacle.process
    local Output = Pinnacle.output
    local Tag = Pinnacle.tag
    local Window = Pinnacle.window

    Input:keybind({ "shift" }, "f", function()
        local focused = Window:get_focused()
        if focused then
            print(focused:fullscreen_or_maximized())
            -- assert(focused:fullscreen_or_maximized() == "neither")
            focused:set_fullscreen(true)
            print(focused:fullscreen_or_maximized())
            -- assert(focused:fullscreen_or_maximized() == "fullscreen")
        end
    end)
end)
