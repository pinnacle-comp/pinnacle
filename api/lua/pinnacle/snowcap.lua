local integration = {}

---The Snowcap widget system, integrated into Pinnacle.
---@class pinnacle.Snowcap
local snowcap = {
    layer = require("snowcap.layer"),
    widget = require("snowcap.widget"),
    input = {
        key = require("snowcap.input.keys"),
    },
    integration = integration,
}

---@class pinnacle.snowcap.integration.QuitPrompt
---@field border_radius number
---@field border_thickness number
---@field background_color snowcap.Color
---@field border_color snowcap.Color
---@field font snowcap.Font
---@field width integer
---@field height integer
local QuitPrompt = {}

---@class pinnacle.snowcap.integration.KeybindOverlay
---@field border_radius number
---@field border_thickness number
---@field background_color snowcap.Color
---@field border_color snowcap.Color
---@field font snowcap.Font
---@field width integer
---@field height integer
local KeybindOverlay = {}

---Show this quit prompt.
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
        border_radius = self.border_radius,
        border_thickness = self.border_thickness,
        border_color = self.border_color,
        background_color = self.background_color,
        child = Widget.column({
            children = {
                Widget.text({
                    text = "Quit Pinnacle?",
                    font = quit_font,
                    size = 20.0,
                }),
                Widget.text({ text = "", size = 8.0 }),
                Widget.text({
                    text = "Press ENTER to confirm, or\nany other key to close this",
                    font = self.font,
                    size = 14.0,
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

    prompt:on_key_press(function(_, key)
        if key == require("snowcap.input.keys").Return then
            require("pinnacle").quit()
        else
            prompt:close()
        end
    end)
end

---Show this keybind overlay.
function KeybindOverlay:show()
    local descriptions = require("pinnacle.input").bind_infos()

    ---@param mods Modifier[]
    ---@param xkb_name string
    ---@return string
    local function keybind_to_string(mods, xkb_name)
        local repr = {}
        for _, mod in ipairs(mods) do
            if mod == "super" then
                table.insert(repr, "Super")
                break
            end
        end
        for _, mod in ipairs(mods) do
            if mod == "ctrl" then
                table.insert(repr, "Ctrl")
                break
            end
        end
        for _, mod in ipairs(mods) do
            if mod == "alt" then
                table.insert(repr, "Alt")
                break
            end
        end
        for _, mod in ipairs(mods) do
            if mod == "shift" then
                table.insert(repr, "Shift")
                break
            end
        end
        table.insert(repr, xkb_name)

        return table.concat(repr, " + ")
    end

    ---@type { group: string?, data: { keybind: string, descs: string[] }[] }[]
    local groups = {}

    for _, desc in ipairs(descriptions) do
        local repr = keybind_to_string(desc.modifiers, desc.xkb_name)

        for _, group in ipairs(groups) do
            if group.group == desc.group then
                for _, keybind in ipairs(group.data) do
                    if keybind.keybind == repr then
                        if desc.description then
                            table.insert(keybind.descs, desc.description)
                        end
                        goto continue
                    end
                end

                table.insert(group.data, { keybind = repr, descs = { desc.description } })
                goto continue
            end
        end

        table.insert(
            groups,
            { group = desc.group, data = { { keybind = repr, descs = { desc.description } } } }
        )

        ::continue::
    end

    -- List keybinds without a group last

    local pos = nil
    for i, group in ipairs(groups) do
        if not group.group then
            pos = i
            break
        end
    end

    if pos then
        local other = table.remove(groups, pos)
        table.insert(groups, other)
    end

    --

    ---@type snowcap.WidgetDef[]
    local sections = {}

    local Widget = require("snowcap.widget")

    local bold_font = require("pinnacle.util").deep_copy(self.font)
    bold_font.weight = Widget.font.weight.BOLD

    for _, group in ipairs(groups) do
        local group_name = group.group or "Other"

        table.insert(sections, Widget.text({ text = group_name, font = bold_font, size = 19.0 }))

        for _, keybind in ipairs(group.data) do
            local repr = keybind.keybind
            local descs = keybind.descs

            if #descs == 0 then
                table.insert(sections, Widget.text({ text = repr, font = self.font }))
            elseif #descs == 1 then
                table.insert(
                    sections,
                    Widget.row({
                        children = {
                            Widget.text({
                                text = repr,
                                width = Widget.length.FillPortion(1),
                                font = self.font,
                            }),
                            Widget.text({
                                text = descs[1],
                                width = Widget.length.FillPortion(2),
                                font = self.font,
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
                        font = self.font,
                    })
                )

                for _, desc in descs do
                    table.insert(
                        children,
                        Widget.text({
                            text = "\t" .. desc,
                            font = self.font,
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

        table.insert(sections, Widget.text({ text = "", size = 8.0 }))
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
                    font = bold_font,
                    size = 24.0,
                    width = Widget.length.Fill,
                }),
                Widget.text({
                    text = "",
                    size = 8.0,
                }),
                scrollable,
            },
        }),
        width = Widget.length.Fill,
        height = Widget.length.Fill,
        padding = {
            top = 16.0,
            left = 16.0,
            bottom = 16.0,
            right = 16.0,
        },
        valign = Widget.alignment.CENTER,
        halign = Widget.alignment.CENTER,
        border_radius = self.border_radius,
        border_color = self.border_color,
        border_thickness = self.border_thickness,
        background_color = self.background_color,
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

    overlay:on_key_press(function(_, _)
        overlay:close()
    end)
end

---Creates the default quit prompt.
---
---Some of its characteristics can be changed by altering its fields.
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

---Creates the default keybind overlay.
---
---Some of its characteristics can be changed by altering its fields.
---@return pinnacle.snowcap.integration.KeybindOverlay
function integration.keybind_overlay()
    local Widget = require("snowcap.widget")

    ---@type pinnacle.snowcap.integration.KeybindOverlay
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

    setmetatable(prompt, { __index = KeybindOverlay })

    return prompt
end

return snowcap
