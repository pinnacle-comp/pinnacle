local M = {}

---Close a window.
---@param client_id integer? The id of the window you want closed, or nil to close the currently focused window, if any.
function M.close_window(client_id)
    SendMsg({
        Action = {
            CloseWindow = {
                client_id = client_id or "nil",
            },
        },
    })
end

return M
