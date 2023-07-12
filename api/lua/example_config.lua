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
    local input = pinnacle.input -- Key and mouse binds
    local window = pinnacle.window -- Window management
    local process = pinnacle.process -- Process spawning
    local tag = pinnacle.tag -- Tag management
    local output = pinnacle.output -- Output management

    -- Every key supported by xkbcommon.
    -- Support for just putting in a string of a key is intended.
    local keys = input.keys

    ---@type Modifier
    local mod_key = "Ctrl" -- This is set to `Ctrl` instead of `Super` to not conflict with your WM/DE keybinds
    -- ^ Add type annotations for that sweet, sweet autocomplete

    local terminal = "alacritty"

    -- Keybinds ----------------------------------------------------------------------

    input.keybind({ mod_key, "Alt" }, keys.q, pinnacle.quit)

    input.keybind({ mod_key, "Alt" }, keys.c, window.close_window)

    input.keybind({ mod_key, "Alt" }, keys.space, window.toggle_floating)

    input.keybind({ mod_key }, keys.Return, function()
        process.spawn(terminal, function(stdout, stderr, exit_code, exit_msg)
            -- do something with the output here
        end)
    end)

    input.keybind({ mod_key }, keys.l, function()
        process.spawn("kitty")
    end)
    input.keybind({ mod_key }, keys.k, function()
        process.spawn("foot")
    end)
    input.keybind({ mod_key }, keys.j, function()
        process.spawn("nautilus")
    end)

    input.keybind({ mod_key }, keys.g, function()
        local op = output.get_by_res(2560, 1440)
        for _, v in pairs(op) do
            print(v.name)
        end
    end)

    -- Tags ---------------------------------------------------------------------------

    output.connect_for_all(function(op)
        tag.add(op, "1", "2", "3", "4", "5")
        tag.toggle("1", op)
    end)

    ---@type Layout[]
    local layouts = { "MasterStack", "Dwindle", "Spiral" }
    local index = 1

    input.keybind({ mod_key }, keys.space, function()
        tag.set_layout("1", layouts[index])
        if index + 1 > #layouts then
            index = 1
        else
            index = index + 1
        end
    end)

    input.keybind({ mod_key }, keys.KEY_1, function()
        tag.switch_to("1")
    end)
    input.keybind({ mod_key }, keys.KEY_2, function()
        tag.switch_to("2")
    end)
    input.keybind({ mod_key }, keys.KEY_3, function()
        tag.switch_to("3")
    end)
    input.keybind({ mod_key }, keys.KEY_4, function()
        tag.switch_to("4")
    end)
    input.keybind({ mod_key }, keys.KEY_5, function()
        tag.switch_to("5")
    end)

    input.keybind({ mod_key, "Shift" }, keys.KEY_1, function()
        tag.toggle("1")
    end)
    input.keybind({ mod_key, "Shift" }, keys.KEY_2, function()
        tag.toggle("2")
    end)
    input.keybind({ mod_key, "Shift" }, keys.KEY_3, function()
        tag.toggle("3")
    end)
    input.keybind({ mod_key, "Shift" }, keys.KEY_4, function()
        tag.toggle("4")
    end)
    input.keybind({ mod_key, "Shift" }, keys.KEY_5, function()
        tag.toggle("5")
    end)

    input.keybind({ mod_key, "Alt" }, keys.KEY_1, function()
        window.get_focused():move_to_tag("1")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_2, function()
        window.get_focused():move_to_tag("2")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_3, function()
        window.get_focused():move_to_tag("3")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_4, function()
        window.get_focused():move_to_tag("4")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_5, function()
        window.get_focused():move_to_tag("5")
    end)

    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_1, function()
        window.get_focused():toggle_tag("1")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_2, function()
        window.get_focused():toggle_tag("2")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_3, function()
        window.get_focused():toggle_tag("3")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_4, function()
        window.get_focused():toggle_tag("4")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_5, function()
        window.get_focused():toggle_tag("5")
    end)
end)
