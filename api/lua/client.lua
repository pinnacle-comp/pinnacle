local M = {}

---Close a window.
---@param client_id integer? The id of the window you want closed, or nil to close the currently focused window, if any.
function M.close_window(client_id)
    SendMsg({
        CloseWindow = {
            client_id = client_id,
        },
    })
end

---Toggle a window's floating status.
---@param client_id integer? The id of the window you want to toggle, or nil to toggle the currently focused window, if any.
function M.toggle_floating(client_id)
    SendMsg({
        ToggleFloating = {
            client_id = client_id,
        },
    })
end

return M
