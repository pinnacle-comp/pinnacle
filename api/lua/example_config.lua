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

    -- Outputs -----------------------------------------------------------------------

    -- You can set your own monitor layout as I have done below for my monitors.

    -- local lg = output.get_by_name("DP-2") --[[@as Output]]
    -- local dell = output.get_by_name("DP-3") --[[@as Output]]
    --
    -- dell:set_loc_left_of(lg, "bottom")

    -- Keybinds ----------------------------------------------------------------------

    -- mod_key + Alt + q quits the compositor
    input.keybind({ mod_key, "Alt" }, keys.q, pinnacle.quit)

    -- mod_key + Alt + c closes the focused window
    input.keybind({ mod_key, "Alt" }, keys.c, function()
        -- The commented out line may crash the config process if you have no windows open.
        -- There is no nil warning here due to limitations in Lua LS type checking, so check for nil as shown below.
        -- window.get_focused():close()
        local win = window.get_focused()
        if win ~= nil then
            win:close()
        end
    end)

    -- mod_key + return spawns a terminal
    input.keybind({ mod_key }, keys.Return, function()
        process.spawn(terminal, function(stdout, stderr, exit_code, exit_msg)
            -- do something with the output here
        end)
    end)

    -- mod_key + Alt + Space toggle floating on the focused window
    input.keybind({ mod_key, "Alt" }, keys.space, function()
        local win = window.get_focused()
        if win ~= nil then
            win:toggle_floating()
        end
    end)

    -- mod_key + f toggles fullscreen on the focused window
    input.keybind({ mod_key }, keys.f, function()
        local win = window.get_focused()
        if win ~= nil then
            win:toggle_fullscreen()
        end
    end)

    -- mod_key + m toggles maximized on the focused window
    input.keybind({ mod_key }, keys.m, function()
        local win = window.get_focused()
        if win ~= nil then
            win:toggle_maximized()
        end
    end)

    -- Tags ---------------------------------------------------------------------------

    output.connect_for_all(function(op)
        -- Add tags 1, 2, 3, 4 and 5 on all monitors, and toggle tag 1 active by default

        op:add_tags("1", "2", "3", "4", "5")
        -- Same as tag.add(op, "1", "2", "3", "4", "5")
        tag.toggle({ name = "1", output = op })

        -- Window rules
        -- Add your own window rules here. Below is an example.
        --
        -- These currently need to be added inside of `connect_for_all` because
        -- it only runs after the whole config is parsed, so any specified tags won't be available outside
        -- of this function. This means that if you have multiple monitors,
        -- these rules will be duplicated unless you write in some logic to prevent that.
        --
        -- window.rules.add({
        --     cond = { class = "kitty" },
        --     rule = { size = { 300, 300 }, location = { 50, 50 } },
        -- }, {
        --     cond = {
        --         class = "XTerm",
        --         tag = "4",
        --     },
        --     rule = { size = { 500, 800 }, floating_or_tiled = "Floating" },
        -- })
    end)

    ---@type Layout[]
    local layouts = {
        "MasterStack",
        "Dwindle",
        "Spiral",
        "CornerTopLeft",
        "CornerTopRight",
        "CornerBottomLeft",
        "CornerBottomRight",
    }
    local indices = {}

    -- Layout cycling
    -- Yes, this is overly complicated and yes, I'll cook up a way to make it less so.
    input.keybind({ mod_key }, keys.space, function()
        local tags = output.get_focused():tags()
        for _, tg in pairs(tags) do
            if tg:active() then
                local name = tg:name()
                if name == nil then
                    return
                end
                tg:set_layout(layouts[indices[name] or 1])
                if indices[name] == nil then
                    indices[name] = 2
                else
                    if indices[name] + 1 > #layouts then
                        indices[name] = 1
                    else
                        indices[name] = indices[name] + 1
                    end
                end
                break
            end
        end
    end)
    input.keybind({ mod_key, "Shift" }, keys.space, function()
        local tags = output.get_focused():tags()
        for _, tg in pairs(tags) do
            if tg:active() then
                local name = tg:name()
                if name == nil then
                    return
                end
                tg:set_layout(layouts[indices[name] or #layouts])
                if indices[name] == nil then
                    indices[name] = #layouts - 1
                else
                    if indices[name] - 1 < 1 then
                        indices[name] = #layouts
                    else
                        indices[name] = indices[name] - 1
                    end
                end
                break
            end
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

    -- I check for nil this way because I don't want stylua to take up like 80 lines on `if win ~= nil`
    input.keybind({ mod_key, "Alt" }, keys.KEY_1, function()
        local _ = window.get_focused() and window:get_focused():move_to_tag("1")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_2, function()
        local _ = window.get_focused() and window:get_focused():move_to_tag("2")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_3, function()
        local _ = window.get_focused() and window:get_focused():move_to_tag("3")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_4, function()
        local _ = window.get_focused() and window:get_focused():move_to_tag("4")
    end)
    input.keybind({ mod_key, "Alt" }, keys.KEY_5, function()
        local _ = window.get_focused() and window:get_focused():move_to_tag("5")
    end)

    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_1, function()
        local _ = window.get_focused() and window.get_focused():toggle_tag("1")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_2, function()
        local _ = window.get_focused() and window.get_focused():toggle_tag("2")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_3, function()
        local _ = window.get_focused() and window.get_focused():toggle_tag("3")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_4, function()
        local _ = window.get_focused() and window.get_focused():toggle_tag("4")
    end)
    input.keybind({ mod_key, "Shift", "Alt" }, keys.KEY_5, function()
        local _ = window.get_focused() and window.get_focused():toggle_tag("5")
    end)
end)
