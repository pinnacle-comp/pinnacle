-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
