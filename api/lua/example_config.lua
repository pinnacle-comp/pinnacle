-- SPDX-License-Identifier: MIT

-- Just like in Awesome, if you want access to Luarocks packages, this needs to be called.
-- NOTE: The loader doesn't load from the local Luarocks directory (probably in ~/.luarocks),
-- |     so if you have any rocks installed with --local,
-- |     you may need to add those paths to package.path and package.cpath.
-- Alternatively, you can add
--     eval $(luarocks path --bin)
-- to your shell's startup script to permanently have access to Luarocks in all your Lua files.
pcall(require, "luarocks.loader")

-- Neovim users be like:
require("pinnacle").setup(function(pinnacle)
    local input = pinnacle.input  --Key and mouse binds
    local client = pinnacle.client --Window management
    local process = pinnacle.process -- Process spawning

    -- Every key supported by xkbcommon.
    -- Support for just putting in a string of a key is intended.
    local keys = input.keys

    -- Keybinds ----------------------------------------------------------------------
    input.keybind({ "Ctrl", "Alt" }, keys.q, pinnacle.quit)

    input.keybind({ "Ctrl", "Alt" }, keys.c, client.close_window)

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
