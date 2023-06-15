local M = {}

---Set a keybind. If called on an already existing keybind, it gets replaced.
---@param key Keys The key for the keybind. NOTE: uppercase and lowercase characters are considered different.
---@param modifiers Modifiers[] Which modifiers need to be pressed for the keybind to trigger.
---@param action function What to run.
function M.keybind(modifiers, key, action)
    table.insert(CallbackTable, action)
    SendMsg({
        SetKeybind = {
            modifiers = modifiers,
            key = key,
            callback_id = #CallbackTable,
        },
    })
end

return M
