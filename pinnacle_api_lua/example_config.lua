local input = require("input")
local client = require("client")

input.keybind({ "Alt", "Ctrl" }, 99, client.close_window)
input.keybind({ "Ctrl", "Alt" }, 32, client.toggle_floating)
