-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")
local render_service = require("pinnacle.grpc.defs").pinnacle.render.v0alpha1.RenderService

---Rendering management.
---
---@class Render
local render = {}

---@alias ScalingFilter
---| "bilinear" Blend between the four closest pixels. May cause scaling to be blurry.
---| "nearest_neighbor" Choose the closest pixel. Causes scaling to look pixelated.

---@type table<ScalingFilter, integer>
local filter_name_to_filter_value = {
    bilinear = 1,
    nearest_neighbor = 2,
}

---Set the upscale filter the renderer will use to upscale buffers.
---
---@param filter ScalingFilter
function render.set_upscale_filter(filter)
    client.unary_request(
        render_service.SetUpscaleFilter,
        { filter = filter_name_to_filter_value[filter] }
    )
end

---Set the downscale filter the renderer will use to downscale buffers.
---
---@param filter ScalingFilter
function render.set_downscale_filter(filter)
    client.unary_request(
        render_service.SetDownscaleFilter,
        { filter = filter_name_to_filter_value[filter] }
    )
end

return render
