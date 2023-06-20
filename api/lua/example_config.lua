require("pinnacle").setup(function(pinnacle)
    local input = pinnacle.input
    local client = pinnacle.client

    input.keybind({ "Alt", "Ctrl" }, 99, client.close_window)
    input.keybind({ "Ctrl", "Alt" }, 32, client.toggle_floating)
end)
