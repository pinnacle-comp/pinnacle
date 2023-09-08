-- SPDX-License-Identifier: GPL-3.0-or-later

---@meta _

---Conditions for window rules. Only one condition can be in the table.
---If you have more than one you need to check for, use `cond_any` or `cond_all`
---to check for any or all conditions.
---@class _WindowRuleCondition
---@field cond_any _WindowRuleCondition[]? At least one provided condition must be true.
---@field cond_all _WindowRuleCondition[]? All provided conditions must be true.
---@field class string[]? The window must have this class.
---@field title string[]? The window must have this title.
---@field tag TagId[]? The window must be on this tag.

---Conditions for window rules. Only one condition can be in the table.
---If you have more than one you need to check for, use `cond_any` or `cond_all`
---to check for any or all conditions.
---@class WindowRuleCondition
---@field cond_any (WindowRuleCondition|WindowRuleCondition[])? At least one provided condition must be true.
---@field cond_all (WindowRuleCondition|WindowRuleCondition[])? All provided conditions must be true.
---@field class (string|string[])? The window must have this class.
---@field title (string|string[])? The window must have this title.
---@field tag (TagConstructor|TagConstructor[])? The window must be on this tag.

---@class _WindowRule Attributes the window will be spawned with.
---@field output OutputName? The output this window will be spawned on. TODO:
---@field tags TagId[]? The tags this window will be spawned with.
---@field floating_or_tiled ("Floating"|"Tiled")? Whether or not this window will be spawned floating or tiled.
---@field fullscreen_or_maximized FullscreenOrMaximized? Whether or not this window will be spawned fullscreen, maximized, or forced to neither.
---@field size { [1]: integer, [2]: integer }? The size the window will spawn with, with [1] being width and [2] being height. This must be a strictly positive integer; putting 0 will crash the compositor.
---@field location { [1]: integer, [2]: integer }? The location the window will spawn at. If the window spawns tiled, it will instead snap to this location when set to floating.

---@class WindowRule Attributes the window will be spawned with.
---@field output (Output|OutputName)? The output this window will be spawned on. TODO:
---@field tags TagConstructor[]? The tags this window will be spawned with.
---@field floating_or_tiled ("Floating"|"Tiled")? Whether or not this window will be spawned floating or tiled.
---@field fullscreen_or_maximized FullscreenOrMaximized? Whether or not this window will be spawned fullscreen, maximized, or forced to neither.
---@field size { [1]: integer, [2]: integer }? The size the window will spawn with, with [1] being width and [2] being height. This must be a strictly positive integer; putting 0 will crash the compositor.
---@field location { [1]: integer, [2]: integer }? The location the window will spawn at. If the window spawns tiled, it will instead snap to this location when set to floating.
