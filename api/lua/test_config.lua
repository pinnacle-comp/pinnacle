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
    local keys = input.keys
    -- Mouse buttons
    local buttons = input.buttons

    ---@type Modifier
    local mod_key = "Ctrl" -- This is set to `Ctrl` instead of `Super` to not conflict with your WM/DE keybinds
    -- ^ Add type annotations for that sweet, sweet autocomplete

    local terminal = "alacritty"

    process.set_env("MOZ_ENABLE_WAYLAND", "1")

    -- Outputs -----------------------------------------------------------------------
    -- You can set your own monitor layout as I have done below for my monitors.
    --
    -- local lg = output.get_by_name("DP-2") --[[@as Output]]
    -- local dell = output.get_by_name("DP-3") --[[@as Output]]
    --
    -- dell:set_loc_left_of(lg, "bottom")

    -- Libinput settings -------------------------------------------------------------
    -- If you want to change settings like pointer acceleration,
    -- you can do them in `input.libinput`.
    --
    -- input.libinput.set_accel_profile("Flat")

    -- Mousebinds --------------------------------------------------------------------

    input.mousebind({ "Ctrl" }, buttons.left, "Press", function()
        window.begin_move(buttons.left)
    end)
    input.mousebind({ "Ctrl" }, buttons.right, "Press", function()
        window.begin_resize(buttons.right)
    end)

    -- Keybinds ----------------------------------------------------------------------

    input.keybind({ mod_key }, keys.t, function()
        window.get_focused():set_size({ w = 500, h = 500 })
    end)

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

    local tags = { "1", "2", "3", "4", "5" }

    output.connect_for_all(function(op)
        -- Add tags 1, 2, 3, 4 and 5 on all monitors, and toggle tag 1 active by default

        op:add_tags(tags)
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

    -- Layout cycling

    -- Create a layout cycler to cycle your tag layouts. This will store which layout each tag has
    -- and change to the next or previous one in the array when the respective function is called.
    local layout_cycler = tag.layout_cycler({
        "MasterStack",
        "Dwindle",
        "Spiral",
        "CornerTopLeft",
        "CornerTopRight",
        "CornerBottomLeft",
        "CornerBottomRight",
    })

    input.keybind({ mod_key }, keys.space, layout_cycler.next)
    input.keybind({ mod_key, "Shift" }, keys.space, layout_cycler.prev)

    -- Tag manipulation

    for _, tag_name in pairs(tags) do
        -- mod_key + 1-5 switches tags
        input.keybind({ mod_key }, tag_name, function()
            tag.switch_to(tag_name)
        end)
        -- mod_key + Shift + 1-5 toggles tags
        input.keybind({ mod_key, "Shift" }, tag_name, function()
            tag.toggle(tag_name)
        end)
        -- mod_key + Alt + 1-5 moves windows to tags
        input.keybind({ mod_key, "Alt" }, tag_name, function()
            local _ = window.get_focused() and window:get_focused():move_to_tag(tag_name)
        end)
        -- mod_key + Shift + Alt + 1-5 toggles tags on windows
        input.keybind({ mod_key, "Shift", "Alt" }, tag_name, function()
            local _ = window.get_focused() and window.get_focused():toggle_tag(tag_name)
        end)
    end
end)
