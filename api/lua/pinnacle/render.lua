-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local render_v1 = require("pinnacle.grpc.defs").pinnacle.render.v1

---Rendering management.
---
---@class pinnacle.render
local render = {}

---@enum (key) pinnacle.render.ScalingFilter
local filter_name_to_filter_value = {
    ---Blend between the four closest pixels. May cause scaling to be blurry.
    bilinear = render_v1.Filter.FILTER_BILINEAR,
    ---Choose the closest pixel. Causes scaling to look pixelated.
    nearest_neighbor = render_v1.Filter.FILTER_NEAREST_NEIGHBOR,
}

---Sets the upscale filter the renderer will use to upscale buffers.
---
---@param filter pinnacle.render.ScalingFilter
function render.set_upscale_filter(filter)
    local _, err = client:pinnacle_render_v1_RenderService_SetUpscaleFilter({
        filter = filter_name_to_filter_value[filter],
    })

    if err then
        log.error(err)
    end
end

---Sets the downscale filter the renderer will use to downscale buffers.
---
---@param filter pinnacle.render.ScalingFilter
function render.set_downscale_filter(filter)
    local _, err = client:pinnacle_render_v1_RenderService_SetDownscaleFilter({
        filter = filter_name_to_filter_value[filter],
    })

    if err then
        log.error(err)
    end
end

return render
