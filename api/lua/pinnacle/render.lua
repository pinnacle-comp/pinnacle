-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client")

---The protobuf absolute path prefix
local prefix = "pinnacle.render." .. client.version .. "."
local service = prefix .. "RenderService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) RenderServiceMethod
local rpc_types = {
    SetUpscaleFilter = {},
    SetDownscaleFilter = {},
}

---Build GrpcRequestParams
---@param method RenderServiceMethod
---@param data table
---@return GrpcRequestParams
local function build_grpc_request_params(method, data)
    local req_type = rpc_types[method].request_type
    local resp_type = rpc_types[method].response_type

    ---@type GrpcRequestParams
    return {
        service = service,
        method = method,
        request_type = req_type and prefix .. req_type or prefix .. method .. "Request",
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---Rendering management.
---
---@class Render
local render = {}

---@alias ScalingFilter
---| "bilinear"
---| "nearest_neighbor"

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
        build_grpc_request_params("SetUpscaleFilter", { filter = filter_name_to_filter_value[filter] })
    )
end

---Set the downscale filter the renderer will use to downscale buffers.
---
---@param filter ScalingFilter
function render.set_downscale_filter(filter)
    client.unary_request(
        build_grpc_request_params("SetDownscaleFilter", { filter = filter_name_to_filter_value[filter] })
    )
end

return render
