-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client_inner = nil

local client = {
    ---@type fun(): grpc_client.Client
    client = function()
        return client_inner
    end,
}

function client.connect()
    client_inner = require("grpc_client").new({
        path = os.getenv("PINNACLE_GRPC_SOCKET"),
    })
end

return client
