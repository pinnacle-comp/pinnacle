require("pinnacle").setup(function(pinnacle)
    local input = pinnacle.input
    local process = pinnacle.process
    local output = pinnacle.output
    local tag = pinnacle.tag
    local window = pinnacle.window

    local mods = input.mod

    input:set_keybind({ mods.SHIFT }, "A", function()
        process:spawn({ "alacritty" }, {
            stdout = function(line)
                print("stdout")
                print(line)
            end,
            stderr = function(line)
                print("stderr")
                print(line)
            end,
            exit = function(code, msg)
                print(code, msg)
            end,
        })
    end)

    input:set_keybind({ 1 }, "Q", function()
        pinnacle:quit()
    end)
end)
