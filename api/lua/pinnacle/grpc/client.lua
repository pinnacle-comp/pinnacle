-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = {
    ---@type grpc_client.Client
    ---@diagnostic disable-next-line: missing-fields
    client = {},
}

function client.connect()
    local c = require("grpc_client").new({
        path = os.getenv("PINNACLE_GRPC_SOCKET"),
    })

    setmetatable(client.client, { __index = c })
end

return client
