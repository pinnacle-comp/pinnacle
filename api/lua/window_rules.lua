---Rules that apply to spawned windows when conditions are met.
---@class WindowRules
local window_rules = {}

---Convert all tag constructors in `cond` to tag ids for serialization.
---@param cond WindowRuleCondition
---@return _WindowRuleCondition
local function convert_tag_params(cond)
    if cond.tag then
        local tags = {}

        if type(cond.tag) == "table" then
            if cond.tag.name or cond.tag.output then
                -- Tag constructor
                local tag = require("tag").get(cond.tag)
                if tag then
                    table.insert(tags, tag:id())
                end
            else
                -- Array of tag constructors
                ---@diagnostic disable-next-line
                for _, t in pairs(cond.tag) do
                    local tag = require("tag").get(t)
                    if tag then
                        table.insert(tags, tag:id())
                    end
                end
            end
        else
            -- Tag constructor
            local tag = require("tag").get(cond.tag)
            if tag then
                table.insert(tags, tag:id())
            end
        end

        cond.tag = tags
    end

    if cond.cond_any then
        local conds = {}
        if type(cond.cond_any[1]) == "table" then
            -- Array of conds
            for _, c in pairs(cond.cond_any) do
                table.insert(conds, convert_tag_params(c))
            end
        else
            -- Single cond
            table.insert(conds, convert_tag_params(cond.cond_any))
        end
        cond.cond_any = conds
    end

    if cond.cond_all then
        local conds = {}
        if type(cond.cond_all[1]) == "table" then
            -- Array of conds
            for _, c in pairs(cond.cond_all) do
                table.insert(conds, convert_tag_params(c))
            end
        else
            -- Single cond
            table.insert(conds, convert_tag_params(cond.cond_all))
        end
        cond.cond_all = conds
    end

    return cond --[[@as _WindowRuleCondition]]
end

---These attributes need to be arrays, so this function converts single values into arrays.
---@param cond WindowRuleCondition
---@return WindowRuleCondition
local function convert_single_attrs(cond)
    if type(cond.class) == "string" then
        -- stylua: ignore start
        cond.class = { cond.class --[[@as string]] }
        -- stylua: ignore end
    end

    if type(cond.title) == "string" then
        -- stylua: ignore start
        cond.title = { cond.title --[[@as string]] }
        -- stylua: ignore end
    end

    if cond.cond_any then
        local conds = {}
        if type(cond.cond_any[1]) == "table" then
            -- Array of conds
            for _, c in pairs(cond.cond_any) do
                table.insert(conds, convert_single_attrs(c))
            end
        else
            -- Single cond
            table.insert(conds, convert_single_attrs(cond.cond_any))
        end
        cond.cond_any = conds
    end

    if cond.cond_all then
        local conds = {}
        if type(cond.cond_all[1]) == "table" then
            -- Array of conds
            for _, c in pairs(cond.cond_all) do
                table.insert(conds, convert_single_attrs(c))
            end
        else
            -- Single cond
            table.insert(conds, convert_single_attrs(cond.cond_all))
        end
        cond.cond_all = conds
    end

    return cond
end

---Add one or more window rules.
---
---A window rule defines what properties a window will spawn with given certain conditions.
---For example, if Firefox is spawned, you can set it to open on a specific tag.
---
---This function takes in a table with two keys:
---
--- - `cond`: The condition for `rule` to apply to a new window.
--- - `rule`: What gets applied to the new window if `cond` is true.
---
---There are some important mechanics you should know when using window rules:
---
--- - All children inside a `cond_all` block must be true for the block to be true.
--- - At least one child inside a `cond_any` block must be true for the block to be true.
--- - The outermost block of a window rule condition is implicitly a `cond_all` block.
--- - All condition attributes (`tag`, `title`, `class`, etc.) can either be a single value or an array.
---   This includes `cond_all` and `cond_any`.
---     - Within a `cond_all` block, any arrays must have all items be true for the attribute to be true.
---     - Within a `cond_any` block, any arrays only need one item to be true for the attribute to be true.
---
---`cond` can be a bit confusing and quite table heavy. Examples are shown below for guidance.
---
---### Examples
---```lua
--- -- A simple window rule. This one will cause Firefox to open on tag "Browser".
---window.rules.add({
---    cond = { class = "firefox" },
---    rule = { tags = { "Browser" } },
---})
---
--- -- To apply rules when *all* provided conditions are true, use `cond_all`.
--- -- `cond_all` takes an array of conditions and checks if all are true.
--- -- The following will open Steam fullscreen only if it opens on tag "5".
---window.rules.add({
---    cond = {
---        cond_all = {
---            class = "steam",
---            tag = tag.get("5"),
---        }
---    },
---    rule = { fullscreen_or_maximized = "Fullscreen" },
---})
---
--- -- The outermost block of a `cond` is implicitly a `cond_all`.
--- -- Thus, the above can be shortened to:
---window.rules.add({
---    cond = {
---        class = "steam",
---        tag = tag.get("5"),
---    },
---    rule = { fullscreen_or_maximized = "Fullscreen" },
---})
---
--- -- `cond_any` also exists to allow at least one provided condition to match.
--- -- The following will open either xterm or Alacritty floating.
---window.rules.add({
---    cond = {
---        cond_any = { class = { "xterm", "Alacritty" } }
---    },
---    rule = { floating_or_tiled = "Floating" }
---})
---
--- -- You can arbitrarily nest `cond_any` and `cond_all` to achieve desired logic.
--- -- The following will open Discord, Thunderbird, or Firefox floating if they
--- -- open on either *all* of tags "A", "B", and "C" or both tags "1" and "2".
---window.rules.add({
---    cond = { cond_all = { -- This outer `cond_all` block is unnecessary, but it's here for clarity.
---        { cond_any = { class = { "firefox", "thunderbird", "discord" } } },
---        { cond_any = {
---            -- Because `tag` is inside a `cond_all` block,
---            -- the window must have all these tags for this to be true.
---            -- If it was in a `cond_any` block, only one tag would need to match.
---            { cond_all = { tag = { "A", "B", "C" } } },
---            { cond_all = { tag = { "1", "2" } } },
---        } }
---    } },
---    rule = { floating_or_tiled = "Floating" },
---})
---```
---@param ... { cond: WindowRuleCondition, rule: WindowRule }
function window_rules.add(...)
    local rules = { ... }

    for _, rule in pairs(rules) do
        rule.cond = convert_single_attrs(rule.cond)

        ---@diagnostic disable-next-line
        rule.cond = convert_tag_params(rule.cond)

        if rule.rule.tags then
            local tags = {}
            for _, tag in pairs(rule.rule.tags) do
                local t = require("tag").get(tag)
                if t then
                    ---@diagnostic disable-next-line
                    t = t:id()
                end
                table.insert(tags, t)
            end
            rule.rule.tags = tags
        end

        if rule.rule.output and type(rule.rule.output) == "table" then
            rule.rule.output = rule
                .rule
                .output--[[@as Output]]
                :name()
        end

        SendMsg({
            AddWindowRule = {
                -- stylua: ignore start
                cond = rule.cond --[[@as _WindowRuleCondition]],
                rule = rule.rule --[[@as _WindowRule]],
                -- stylua: ignore end
            },
        })
    end
end

return window_rules
