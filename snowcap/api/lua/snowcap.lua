-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("snowcap.grpc.client").client

---@class snowcap.Snowcap
local snowcap = {
    layer = require("snowcap.layer"),
    widget = require("snowcap.widget"),
}

function snowcap.init()
    require("snowcap.grpc.protobuf").build_protos()
    require("snowcap.grpc.client").connect()
end

function snowcap.listen()
    local success, err = client().loop:loop()
    if not success then
        print(err)
    end
end

---@param setup_fn fun(snowcap: snowcap.Snowcap)
function snowcap.setup(setup_fn)
    snowcap.init()

    setup_fn(snowcap)

    snowcap.listen()
end

return snowcap
