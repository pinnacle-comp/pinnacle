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
---@class pinnacle.snowcap.integration.QuitPrompt
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
---@class pinnacle.snowcap.integration.BindOverlay
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

---Shows this quit prompt.
function QuitPrompt:show()
    local Widget = require("snowcap.widget")
    local Layer = require("snowcap.layer")

    local quit_font = require("pinnacle.util").deep_copy(self.font)
    quit_font.weight = Widget.font.weight.BOLD

    local prompt = Widget.container({
        width = Widget.length.Fill,
        height = Widget.length.Fill,
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

    local prompt = Layer.new_widget({
        widget = prompt,
        width = self.width,
        height = self.height,
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

---Shows this bind overlay.
function BindOverlay:show()
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
        width = Widget.length.Fill,
        height = Widget.length.Fill,
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

    local Layer = require("snowcap.layer")

    local overlay = Layer.new_widget({
        widget = overlay,
        width = self.width,
        height = self.height,
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

return snowcap
