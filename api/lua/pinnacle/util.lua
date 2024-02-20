-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---Utility functions.
---@class Util
local util = {}

---Batch a set of requests that will be sent to the compositor all at once.
---
---Normally, all API calls are blocking. For example, calling `Window.get_all`
---then calling `WindowHandle.props` on each returned window handle will block
---after each `props` call waiting for the compositor to respond:
---
---```
---local handles = Window.get_all()
---
--- -- Collect all the props into this table
---local props = {}
---
--- -- This for loop will block after each call. If the compositor is running slowly
--- -- for whatever reason, this will take a long time to complete as it requests
--- -- properties sequentially.
---for i, handle in ipairs(handles) do
---    props[i] = handle:props()
---end
---```
---
---In order to mitigate this issue, you can batch up a set of API calls using this function.
---This will send all requests to the compositor at once without blocking, then wait for the compositor
---to respond.
---
---You must wrap each request in a function, otherwise they would just get
---evaluated at the callsite in a blocking manner.
---
---### Example
---```lua
---local handles = window.get_all()
---
--- ---@type (fun(): WindowProperties)[]
---local requests = {}
---
--- -- Wrap each request to `props` in another function
---for i, handle in ipairs(handles) do
---    requests[i] = function()
---        return handle:props()
---    end
---end
---
--- -- Batch send these requests
---local props = require("pinnacle.util").batch(requests)
--- -- `props` now contains the `WindowProperties` of all the windows above
---```
---
---@generic T
---
---@param requests (fun(): T)[] The requests that you want to batch up, wrapped in a function.
---
---@return T[] responses The results of each request in the same order that they were in `requests`.
function util.batch(requests)
    local loop = require("cqueues").new()

    local responses = {}

    for i, request in ipairs(requests) do
        loop:wrap(function()
            responses[i] = request()
        end)
    end

    loop:loop()

    return responses
end

return util
