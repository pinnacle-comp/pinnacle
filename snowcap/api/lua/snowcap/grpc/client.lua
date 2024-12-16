-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = {
    ---@type grpc_client.Client
    ---@diagnostic disable-next-line: missing-fields
    client = {},
}

local function socket_path()
    local dir = os.getenv("XDG_RUNTIME_DIR")
    if not dir then
        print("$XDG_RUNTIME_DIR not set, exiting")
        os.exit(1)
    end

    local wayland_instance = os.getenv("WAYLAND_DISPLAY") or "wayland-0"

    local path = dir .. "/snowcap-grpc-" .. wayland_instance .. ".sock"

    return path
end

function client.connect()
    local c = require("grpc_client").new({
        path = socket_path(),
    })

    setmetatable(client.client, { __index = c })
end

return client
