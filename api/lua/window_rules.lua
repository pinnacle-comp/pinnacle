---Rules that apply to spawned windows when conditions are met.
---@class WindowRules
local window_rules = {}

---Add one or more window rules.
---
---A window rule defines what a window will spawn with given certain conditions.
---For example, if Firefox is spawned, you can set it to open on the second tag.
---
---This function takes in a table with two keys:
--- - `cond`: The condition for `rule` to apply to a new window.
--- - `rule`: What gets applied to the new window if `cond` is true.
---
---`cond` can be a bit confusing and *very* table heavy. Examples are shown below for guidance.
---An attempt at simplifying this API will happen in the future, but is a low priority.
---
---### Examples
---```lua
----- A simple window rule. This one will cause Firefox to open on tag "Browser".
---window.rules.add({
---    cond = { class = "firefox" },
---    rule = { tags = { "Browser" } },
---})
---
----- To apply rules when *all* provided conditions are true, use `cond_all`.
----- `cond_all` takes an array of conditions and checks if all are true.
----- Note that `cond_any` is not a keyed table; rather, it's a table of tables.
---
----- The following will open Steam fullscreen only if it opens on tag "5".
---window.rules.add({
---    cond = {
---        cond_any = {
---            { class = "steam" }, -- Note that each table must only have one key.
---            { tag = tag.get_by_name("5")[1] },
---        }
---    },
---    rule = { fullscreen_or_maximized = "Fullscreen" },
---})
---
----- You can arbitrarily nest `cond_any` and `cond_all` to achieve desired logic.
----- The following will open Discord, Thunderbird, or Alacritty floating if they
----- open on either *all* of tags "A", "B", and "C" or both tags "1" and "2".
---window.rules.add({
---    cond = { cond_all = {
---        { cond_any = { { class = "discord" }, { class = "firefox" }, { class = "thunderbird" } } },
---        { cond_any = {
---            { cond_all = { { tag = "A" }, { tag = "B" }, { tag = "C" } } },
---            { cond_all = { { tag = "1" }, { tag = "2" } } },
---        } }
---    } },
---    rule = { floating_or_tiled = "Floating" },
---})
---```
---@param ... { cond: WindowRuleCondition, rule: WindowRule }
function window_rules.add(...)
    local rules = { ... }

    for _, rule in pairs(rules) do
        SendMsg({
            AddWindowRule = {
                cond = rule.cond,
                rule = rule.rule,
            },
        })
    end
end

return window_rules
