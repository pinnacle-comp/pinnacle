-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local render_v0alpha1 = require("pinnacle.grpc.defs").pinnacle.render.v0alpha1
local render_service = require("pinnacle.grpc.defs").pinnacle.render.v0alpha1.RenderService

---Rendering management.
---
---@class Render
local render = {}

---@enum (key) ScalingFilter
local filter_name_to_filter_value = {
    ---Blend between the four closest pixels. May cause scaling to be blurry.
    bilinear = render_v0alpha1.Filter.FILTER_BILINEAR,
    ---Choose the closest pixel. Causes scaling to look pixelated.
    nearest_neighbor = render_v0alpha1.Filter.FILTER_NEAREST_NEIGHBOR,
}

---Set the upscale filter the renderer will use to upscale buffers.
---
---@param filter ScalingFilter
function render.set_upscale_filter(filter)
    client:unary_request(
        render_service.SetUpscaleFilter,
        { filter = filter_name_to_filter_value[filter] }
    )
end

---Set the downscale filter the renderer will use to downscale buffers.
---
---@param filter ScalingFilter
function render.set_downscale_filter(filter)
    client:unary_request(
        render_service.SetDownscaleFilter,
        { filter = filter_name_to_filter_value[filter] }
    )
end

return render
