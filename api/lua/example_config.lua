require("pinnacle").setup(function(pinnacle)
    local input = pinnacle.input
    local client = pinnacle.client
    local keys = pinnacle.keys
    local process = pinnacle.process

    input.keybind({ "Alt", "Ctrl" }, keys.c, client.close_window)
    input.keybind({ "Ctrl", "Alt" }, keys.space, client.toggle_floating)

    input.keybind({ "Ctrl" }, keys.Return, function()
        process.spawn("alacritty", function(stdout, stderr, exit_code, exit_msg)
            -- do something with the output here
        end)
    end)

    input.keybind({ "Ctrl" }, keys.KEY_1, function()
        process.spawn("kitty")
    end)
    input.keybind({ "Ctrl" }, keys.KEY_2, function()
        process.spawn("foot")
    end)
    input.keybind({ "Ctrl" }, keys.KEY_3, function()
        process.spawn("nautilus")
    end)
end)
