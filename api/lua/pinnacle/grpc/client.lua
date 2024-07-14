-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")

local client = {
    ---@type grpc_client.Client
    ---@diagnostic disable-next-line: missing-fields
    client = {},
}

function client.connect()
    local socket_path = os.getenv("PINNACLE_GRPC_SOCKET")

    if not socket_path then
        error("`PINNACLE_GRPC_SOCKET` was not set; is Pinnacle running?")
    end

    local c = require("grpc_client").new({
        path = socket_path,
    })

    log:info("Connected to socket at " .. socket_path)

    setmetatable(client.client, { __index = c })
end

return client
