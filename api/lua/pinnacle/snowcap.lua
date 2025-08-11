-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- I love side effects
do
    local success, snowcap = pcall(require, "snowcap")
    if not success then
        return false
    end

    local success, err = pcall(snowcap.init)

    if not success then
        print("failed to init snowcap: " .. err)
        return false
    end
end

---Builtins and integrations with Snowcap.
---@class pinnacle.snowcap.integration
local integration = {}

---The Snowcap widget system, integrated into Pinnacle.
---@class pinnacle.snowcap
local snowcap = {
    ---Builtins and integrations with Snowcap.
    integration = integration,
}

---A quit prompt.
---
---When opened, pressing ENTER will quit the compositor.
---@class pinnacle.snowcap.integration.QuitPrompt : snowcap.widget.Program
---The radius of the prompt's corners.
---@field border_radius number
---THe thickness of the prompt border.
---@field border_thickness number
---The color of the prompt background.
---@field background_color snowcap.widget.Color
---The color of the prompt border.
---@field border_color snowcap.widget.Color
---The font of the prompt.
---@field font snowcap.widget.Font
---The width of the prompt.
---@field width integer
---The height of the prompt.
---@field height integer
local QuitPrompt = {}

---An overlay that shows various input binds.
---@class pinnacle.snowcap.integration.BindOverlay : snowcap.widget.Program
---The radius of the overlay's corners.
---@field border_radius number
---The thickness of the overlay border.
---@field border_thickness number
---The color of the overlay background.
---@field background_color snowcap.widget.Color
---The color of the overlay border.
---@field border_color snowcap.widget.Color
---The font of the overlay.
---@field font snowcap.widget.Font
---The width of the overlay.
---@field width integer
---The height of the overlay.
---@field height integer
local BindOverlay = {}

---A border that shows window focus, with an optional titlebar.
---@class pinnacle.snowcap.integration.FocusBorder : snowcap.widget.Program
---The window this border is decorating.
---@field window pinnacle.window.WindowHandle
---The thickness of the border, in pixels.
---@field thickness integer
---The color of the border when it's focused.
---@field focused_color snowcap.widget.Color
---The color of the border when it's unfocused.
---@field unfocused_color snowcap.widget.Color
---Whether the window this border surrounds is focused.
---@field focused boolean
---Whether to draw a titlebar
---@field include_titlebar boolean
---The height of the titlebar
---@field titlebar_height integer
local FocusBorder = {}

function QuitPrompt:view()
    local Widget = require("snowcap.widget")

    local quit_font = require("pinnacle.util").deep_copy(self.font)
    quit_font.weight = Widget.font.weight.BOLD

    local prompt = Widget.container({
        width = Widget.length.Fixed(self.width),
        height = Widget.length.Fixed(self.height),
        valign = Widget.alignment.CENTER,
        halign = Widget.alignment.CENTER,
        style = {
            background_color = self.background_color,
            border = {
                width = self.border_thickness,
                color = self.border_color,
                radius = {
                    top_left = self.border_radius,
                    top_right = self.border_radius,
                    bottom_left = self.border_radius,
                    bottom_right = self.border_radius,
                },
            },
        },
        child = Widget.column({
            children = {
                Widget.text({
                    text = "Quit Pinnacle?",
                    style = {
                        font = quit_font,
                        pixels = 20.0,
                    },
                }),
                Widget.text({ text = "", style = { pixels = 8.0 } }),
                Widget.text({
                    text = "Press ENTER to confirm, or\nany other key to close this",
                    style = {
                        font = self.font,
                        pixels = 14.0,
                    },
                }),
            },
        }),
    })

    return prompt
end

function QuitPrompt:update(_) end

---Shows this quit prompt.
function QuitPrompt:show()
    local Layer = require("snowcap.layer")
    local prompt = Layer.new_widget({
        program = self,
        anchor = nil,
        keyboard_interactivity = Layer.keyboard_interactivity.EXCLUSIVE,
        exclusive_zone = "respect",
        layer = Layer.zlayer.OVERLAY,
    })

    if not prompt then
        return
    end

    prompt:on_key_press(function(_, key)
        if key == require("snowcap.input.keys").Return then
            require("pinnacle").quit()
        else
            prompt:close()
        end
    end)
end

function BindOverlay:view()
    ---@param mods pinnacle.input.Mod[]
    ---@return string?
    local function mods_to_string(mods)
        local repr = {}
        local mod_mask = {}
        for _, mod in ipairs(mods) do
            if mod == "super" then
                mod_mask.super = true
            elseif mod == "shift" then
                mod_mask.shift = true
            elseif mod == "ctrl" then
                mod_mask.ctrl = true
            elseif mod == "alt" then
                mod_mask.alt = true
            elseif mod == "iso_level3_shift" then
                mod_mask.iso_level3_shift = true
            elseif mod == "iso_level5_shift" then
                mod_mask.iso_level5_shift = true
            end
        end

        if mod_mask.super then
            table.insert(repr, "Super")
        end
        if mod_mask.ctrl then
            table.insert(repr, "Ctrl")
        end
        if mod_mask.alt then
            table.insert(repr, "Alt")
        end
        if mod_mask.shift then
            table.insert(repr, "Shift")
        end
        if mod_mask.iso_level3_shift then
            table.insert(repr, "ISO Level 3 Shift")
        end
        if mod_mask.iso_level5_shift then
            table.insert(repr, "ISO Level 5 Shift")
        end

        if #repr == 0 then
            return nil
        end

        return table.concat(repr, " + ")
    end

    ---@param mods pinnacle.input.Mod[]
    ---@param key_or_button_name string
    ---@param layer string?
    ---@return string
    local function key_or_mousebind_to_string(mods, key_or_button_name, layer)
        local repr = {}
        if layer then
            table.insert(repr, "[" .. layer .. "] ")
        end
        local mods = mods_to_string(mods)
        if mods then
            table.insert(repr, mods)
            table.insert(repr, " + ")
        end
        table.insert(repr, key_or_button_name)

        return table.concat(repr)
    end

    local bind_infos = require("pinnacle.input").bind_infos()

    ---@type { group: string, keybinds: { keybind: string, descs: string[] }[], mousebinds: { mousebind: string, descs: string[] }[] }[]
    local groups = {}

    for _, bind_info in ipairs(bind_infos) do
        local bind_group = nil

        local has_group = false
        for _, group in ipairs(groups) do
            if group.group == bind_info.group then
                has_group = true
                bind_group = group
                break
            end
        end

        if not has_group then
            table.insert(groups, { group = bind_info.group, keybinds = {}, mousebinds = {} })
            bind_group = groups[#groups]
        end

        assert(bind_group)

        if bind_info.kind.key then
            local repr = key_or_mousebind_to_string(bind_info.mods, bind_info.kind.key.xkb_name)
            for _, keybind in ipairs(bind_group.keybinds) do
                if keybind.keybind == repr then
                    if bind_info.description:len() > 0 then
                        table.insert(keybind.descs, bind_info.description)
                    end
                    goto continue
                end
            end

            table.insert(bind_group.keybinds, { keybind = repr, descs = { bind_info.description } })
        elseif bind_info.kind.mouse then
            local repr = key_or_mousebind_to_string(bind_info.mods, bind_info.kind.mouse.button)
            for _, mousebind in ipairs(bind_group.mousebinds) do
                if mousebind.mousebind == repr then
                    if bind_info.description:len() > 0 then
                        table.insert(mousebind.descs, bind_info.description)
                    end
                    goto continue
                end
            end

            table.insert(
                bind_group.mousebinds,
                { mousebind = repr, descs = { bind_info.description } }
            )
        end

        ::continue::
    end

    -- List keybinds without a group last

    local pos = nil
    for i, group in ipairs(groups) do
        if group.group:len() == 0 then
            pos = i
            break
        end
    end

    if pos then
        local other = table.remove(groups, pos)
        table.insert(groups, other)
    end

    --

    ---@type snowcap.widget.WidgetDef[]
    local sections = {}

    local Widget = require("snowcap.widget")

    local bold_font = require("pinnacle.util").deep_copy(self.font)
    bold_font.weight = Widget.font.weight.BOLD

    for _, group in ipairs(groups) do
        local group_name = group.group
        if group_name:len() == 0 then
            group_name = "Other"
        end

        table.insert(
            sections,
            Widget.text({
                text = group_name,
                style = {
                    font = bold_font,
                    pixels = 19.0,
                },
            })
        )

        for _, keybind in ipairs(group.keybinds) do
            local repr = keybind.keybind
            local descs = keybind.descs

            if #descs == 0 then
                table.insert(
                    sections,
                    Widget.text({
                        text = repr,
                        style = {
                            font = self.font,
                        },
                    })
                )
            elseif #descs == 1 then
                table.insert(
                    sections,
                    Widget.row({
                        children = {
                            Widget.text({
                                text = repr,
                                width = Widget.length.FillPortion(1),
                                style = {
                                    font = self.font,
                                },
                            }),
                            Widget.text({
                                text = descs[1],
                                width = Widget.length.FillPortion(2),
                                style = {
                                    font = self.font,
                                },
                            }),
                        },
                    })
                )
            else
                local children = {}

                table.insert(
                    children,
                    Widget.text({
                        text = repr .. ":",
                        style = {
                            font = self.font,
                        },
                    })
                )

                for _, desc in ipairs(descs) do
                    table.insert(
                        children,
                        Widget.text({
                            text = "\t" .. desc,
                            style = {
                                font = self.font,
                            },
                        })
                    )
                end

                table.insert(
                    sections,
                    Widget.column({
                        children = children,
                    })
                )
            end
        end

        for _, mousebind in ipairs(group.mousebinds) do
            local repr = mousebind.mousebind
            local descs = mousebind.descs

            if #descs == 0 then
                table.insert(
                    sections,
                    Widget.text({
                        text = repr,
                        style = {
                            font = self.font,
                        },
                    })
                )
            elseif #descs == 1 then
                table.insert(
                    sections,
                    Widget.row({
                        children = {
                            Widget.text({
                                text = repr,
                                width = Widget.length.FillPortion(1),
                                style = {
                                    font = self.font,
                                },
                            }),
                            Widget.text({
                                text = descs[1],
                                width = Widget.length.FillPortion(2),
                                style = {
                                    font = self.font,
                                },
                            }),
                        },
                    })
                )
            else
                local children = {}

                table.insert(
                    children,
                    Widget.text({
                        text = repr .. ":",
                        style = {
                            font = self.font,
                        },
                    })
                )

                for _, desc in ipairs(descs) do
                    table.insert(
                        children,
                        Widget.text({
                            text = "\t" .. desc,
                            style = {
                                font = self.font,
                            },
                        })
                    )
                end

                table.insert(
                    sections,
                    Widget.column({
                        children = children,
                    })
                )
            end
        end

        table.insert(
            sections,
            Widget.text({
                text = "",
                style = { pixels = 8.0 },
            })
        )
    end

    local scrollable = Widget.scrollable({
        child = Widget.column({
            children = sections,
        }),
        width = Widget.length.Fill,
        height = Widget.length.Fill,
    })

    local overlay = Widget.container({
        child = Widget.column({
            children = {
                Widget.text({
                    text = "Keybinds",
                    style = {
                        font = bold_font,
                        pixels = 24.0,
                    },
                    width = Widget.length.Fill,
                }),
                Widget.text({
                    text = "",
                    style = {
                        pixels = 8.0,
                    },
                }),
                scrollable,
            },
        }),
        width = Widget.length.Fixed(self.width),
        height = Widget.length.Fixed(self.height),
        padding = {
            top = self.border_thickness + 10.0,
            left = self.border_thickness + 10.0,
            right = self.border_thickness + 10.0,
            bottom = self.border_thickness + 10.0,
        },
        valign = Widget.alignment.CENTER,
        halign = Widget.alignment.CENTER,
        style = {
            background_color = self.background_color,
            border = {
                width = self.border_thickness,
                color = self.border_color,
                radius = {
                    top_left = self.border_radius,
                    top_right = self.border_radius,
                    bottom_left = self.border_radius,
                    bottom_right = self.border_radius,
                },
            },
        },
    })
    return overlay
end

function BindOverlay:update(_) end

---Shows this bind overlay.
function BindOverlay:show()
    local Layer = require("snowcap.layer")

    local overlay = Layer.new_widget({
        program = self,
        anchor = nil,
        keyboard_interactivity = Layer.keyboard_interactivity.EXCLUSIVE,
        exclusive_zone = "respect",
        layer = Layer.zlayer.OVERLAY,
    })

    if not overlay then
        return
    end

    overlay:on_key_press(function(_, _)
        overlay:close()
    end)
end

local B = "\0\0\0\255"
local T = "\0\0\0\0"

-- stylua: ignore
local exit_icon = table.concat({
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,T,T,
    T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,
    T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,
    T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,
    T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,
    T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,
    T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,
    T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,
    T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,T,
    T,T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,T,
    T,T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,T,
    T,T,B,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,B,T,T,
    T,T,B,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T
})

-- stylua: ignore
local maximize_icon = table.concat({
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,T,T,
    T,T,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,B,B,B,T,T,
    T,T,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,T,T,
    T,T,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,T,T,
    T,T,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,B,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,
    T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T,T
})

function FocusBorder:view()
    local Widget = require("snowcap.widget")

    local function brighten(amt)
        local color = self.focused and self.focused_color or self.unfocused_color
        color = require("pinnacle.util").deep_copy(color)
        color.red = color.red + amt
        color.green = color.green + amt
        color.blue = color.blue + amt
        return color
    end

    local children = {}

    if self.include_titlebar then
        local titlebar = Widget.container({
            style = {
                background_color = self.focused and self.focused_color or self.unfocused_color,
            },
            padding = {
                top = self.thickness,
                right = self.thickness,
                bottom = 0,
                left = self.thickness,
            },
            child = Widget.row({
                item_alignment = Widget.alignment.START,
                spacing = 4,
                width = Widget.length.Fill,
                height = Widget.length.Fixed(self.titlebar_height),
                children = {
                    Widget.text({
                        text = "TITLE GOES HERE",
                        style = {
                            pixels = self.titlebar_height - 2,
                        },
                        width = Widget.length.Fill,
                    }),
                    Widget.button({
                        width = Widget.length.Fixed(self.titlebar_height),
                        height = Widget.length.Fixed(self.titlebar_height),
                        padding = {
                            top = 4,
                            bottom = 4,
                            right = 4,
                            left = 4,
                        },
                        style = {
                            active = {
                                background_color = brighten(0.3),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                            hovered = {
                                background_color = brighten(0.4),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                            pressed = {
                                background_color = brighten(0.5),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                        },
                        on_press = "maximize",
                        child = Widget.Image({
                            handle = {
                                rgba = {
                                    width = 32,
                                    height = 32,
                                    rgba = maximize_icon,
                                },
                            },
                            width = Widget.length.Fill,
                            height = Widget.length.Fill,
                        }),
                    }),
                    Widget.button({
                        width = Widget.length.Fixed(self.titlebar_height),
                        height = Widget.length.Fixed(self.titlebar_height),
                        padding = {
                            top = 4,
                            bottom = 4,
                            right = 4,
                            left = 4,
                        },
                        style = {
                            active = {
                                background_color = brighten(0.3),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                            hovered = {
                                background_color = brighten(0.4),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                            pressed = {
                                background_color = brighten(0.5),
                                border = {
                                    radius = {
                                        bottom_left = 1000,
                                        bottom_right = 1000,
                                        top_left = 1000,
                                        top_right = 1000,
                                    },
                                },
                            },
                        },
                        on_press = "close",
                        child = Widget.Image({
                            handle = {
                                rgba = {
                                    width = 32,
                                    height = 32,
                                    rgba = exit_icon,
                                },
                            },
                            width = Widget.length.Fill,
                            height = Widget.length.Fill,
                        }),
                    }),
                },
            }),
        })

        table.insert(children, titlebar)
    end

    table.insert(
        children,
        Widget.container({
            width = Widget.length.Fill,
            height = Widget.length.Fill,
            padding = {
                left = self.thickness,
                right = self.thickness,
                bottom = self.thickness,
                top = self.thickness,
            },
            style = {
                background_color = {
                    red = 0,
                    green = 0,
                    blue = 0,
                    alpha = 0,
                },
                border = {
                    color = self.focused and self.focused_color or self.unfocused_color,
                    width = self.thickness,
                    radius = {
                        top_right = 0,
                        top_left = 0,
                        bottom_right = 0,
                        bottom_left = 0,
                    },
                },
            },
            child = Widget.input_region({
                width = Widget.length.Fill,
                height = Widget.length.Fill,
                add = false,
                child = Widget.row({
                    children = {},
                }),
            }),
        })
    )

    local col = Widget.column({
        children = children,
    })

    return col
end

function FocusBorder:update(msg)
    if msg == true then
        self.focused = true
    elseif msg == false then
        self.focused = false
    elseif msg == "maximize" then
        self.window:toggle_maximized()
    elseif msg == "close" then
        self.window:close()
    end
end

---Decorates the window with this focus border.
---
---@return snowcap.decoration.DecorationHandle|nil
function FocusBorder:decorate()
    local Deco = require("snowcap.decoration")

    local border = Deco.new_widget({
        program = self,
        toplevel_identifier = self.window:foreign_toplevel_list_identifier() or "",
        bounds = {
            left = self.thickness,
            right = self.thickness,
            top = self.thickness * 2 + self.titlebar_height,
            bottom = self.thickness,
        },
        extents = {
            left = self.thickness,
            right = self.thickness,
            top = self.thickness * 2 + self.titlebar_height,
            bottom = self.thickness,
        },
        z_index = 20,
    })

    if not border then
        return nil
    end

    ---@type pinnacle.signal.SignalHandles[]
    local signal_holder = {}

    local signal = require("pinnacle.window").connect_signal({
        focused = function(focused)
            if self.window:foreign_toplevel_list_identifier() then
                border:send_message(self.window.id == focused.id)
            else
                signal_holder[1]:disconnect_all()
            end
        end,
    })

    signal_holder[1] = signal

    return border
end

---Creates the default quit prompt.
---
---Some of its characteristics can be changed by altering its fields.
---
---@return pinnacle.snowcap.integration.QuitPrompt
function integration.quit_prompt()
    local Widget = require("snowcap.widget")

    ---@type pinnacle.snowcap.integration.QuitPrompt
    local prompt = {
        border_radius = 12.0,
        border_thickness = 6.0,
        background_color = Widget.color.from_rgba(0.15, 0.03, 0.1, 0.65),
        border_color = Widget.color.from_rgba(0.8, 0.2, 0.4),
        font = {
            family = Widget.font.family.Name("Ubuntu"),
        },
        width = 220,
        height = 120,
    }

    setmetatable(prompt, { __index = QuitPrompt })

    return prompt
end

---Creates the default bind overlay.
---
---Some of its characteristics can be changed by altering its fields.
---
---@return pinnacle.snowcap.integration.BindOverlay
function integration.bind_overlay()
    local Widget = require("snowcap.widget")

    ---@type pinnacle.snowcap.integration.BindOverlay
    local prompt = {
        border_radius = 12.0,
        border_thickness = 6.0,
        background_color = Widget.color.from_rgba(0.15, 0.15, 0.225, 0.8),
        border_color = Widget.color.from_rgba(0.4, 0.4, 0.7),
        font = {
            family = Widget.font.family.Name("Ubuntu"),
        },
        width = 700,
        height = 500,
    }

    setmetatable(prompt, { __index = BindOverlay })

    return prompt
end

---Creates the default focus border without a titlebar.
---
---@param window pinnacle.window.WindowHandle
---
---@return pinnacle.snowcap.integration.FocusBorder
function integration.focus_border(window)
    local Widget = require("snowcap.widget")

    ---@type pinnacle.snowcap.integration.FocusBorder
    local border = {
        window = window,
        thickness = 4,
        focused_color = Widget.color.from_rgba(0.4, 0.15, 0.7),
        unfocused_color = Widget.color.from_rgba(0.15, 0.15, 0.15),
        focused = false,
        include_titlebar = false,
        titlebar_height = 0,
    }

    setmetatable(border, { __index = FocusBorder })

    return border
end

---Creates the default focus border with a titlebar.
---
---@param window pinnacle.window.WindowHandle
---
---@return pinnacle.snowcap.integration.FocusBorder
function integration.focus_border_with_titlebar(window)
    local Widget = require("snowcap.widget")

    ---@type pinnacle.snowcap.integration.FocusBorder
    local border = {
        window = window,
        thickness = 4,
        focused_color = Widget.color.from_rgba(0.4, 0.15, 0.7),
        unfocused_color = Widget.color.from_rgba(0.15, 0.15, 0.15),
        focused = false,
        include_titlebar = true,
        titlebar_height = 16,
    }

    setmetatable(border, { __index = FocusBorder })

    return border
end

return snowcap
