local client = require("pinnacle.grpc.client")
local protobuf = require("pinnacle.grpc.protobuf")

---The protobuf absolute path prefix
local prefix = "pinnacle.layout." .. client.version .. "."
local service = prefix .. "LayoutService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) LayoutServiceMethod
local rpc_types = {
    Layout = {
        response_type = "LayoutResponse",
    },
}

---Build GrpcRequestParams
---@param method LayoutServiceMethod
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

---@class LayoutArgs
---@field output OutputHandle
---@field windows WindowHandle[]
---@field tags TagHandle[]
---@field output_width integer
---@field output_height integer

---@class Builtin
---@field layout fun(self: self, args: LayoutArgs): { x: integer, y: integer, width: integer, height: integer }[]

---@class Builtin.MasterStack : Builtin
---Gaps between windows, in pixels.
---
---This can be an integer or the table { inner: integer, outer: integer }.
---If it is an integer, all gaps will be that amount of pixels wide.
---If it is a table, `outer` denotes the amount of pixels around the
---edge of the output area that will become a gap, and
---`inner` denotes the amount of pixels around each window that
---will become a gap.
---
---This means that, for example, `inner = 2` will cause the gap
---width between windows to be 4; 2 around each window.
---
---Defaults to 4.
---@field gaps integer | { inner: integer, outer: integer }
---The proportion of the output taken up by the master window(s).
---
---This is a float that will be clamped between 0.1 and 0.9
---similarly to River.
---
---Defaults to 0.5.
---@field master_factor number
---The side the master window(s) will be on.
---
---Defaults to `"left"`.
---@field master_side "left"|"right"|"top"|"bottom"
---How many windows the master side will have.
---
---Defaults to 1.
---@field master_count integer

local builtins = {
    ---@type Builtin.MasterStack
    master_stack = {
        gaps = 4,
        master_factor = 0.5,
        master_side = "left",
        master_count = 1,
    },
}

---@param args LayoutArgs
---
---@return { x: integer, y: integer, width: integer, height: integer }[]
function builtins.master_stack:layout(args)
    local win_count = #args.windows

    if win_count == 0 then
        return {}
    end

    local width = args.output_width
    local height = args.output_height

    ---@type { x: integer, y: integer, width: integer, height: integer }[]
    local geos = {}

    local master_factor = math.max(math.min(self.master_factor, 0.9), 0.1)
    if win_count <= self.master_count then
        master_factor = 1
    end

    local rect = require("pinnacle.util").rectangle.new(0, 0, width, height)

    local master_rect
    local stack_rect

    if type(self.gaps) == "number" then
        local gaps = self.gaps --[[@as integer]]

        rect = rect:split_at("horizontal", 0, gaps)
        rect = rect:split_at("horizontal", height - gaps, gaps)
        rect = rect:split_at("vertical", 0, gaps)
        rect = rect:split_at("vertical", width - gaps, gaps)

        if self.master_side == "left" then
            master_rect, stack_rect = rect:split_at("vertical", math.floor(width * master_factor) - gaps // 2, gaps)
        elseif self.master_side == "right" then
            stack_rect, master_rect = rect:split_at("vertical", math.floor(width * master_factor) - gaps // 2, gaps)
        elseif self.master_side == "top" then
            master_rect, stack_rect = rect:split_at("horizontal", math.floor(height * master_factor) - gaps // 2, gaps)
        else
            stack_rect, master_rect = rect:split_at("horizontal", math.floor(height * master_factor) - gaps // 2, gaps)
        end

        if not master_rect then
            assert(stack_rect)
            master_rect = stack_rect
            stack_rect = nil
        end

        local master_slice_count
        local stack_slice_count = nil

        if win_count > self.master_count then
            master_slice_count = self.master_count - 1
            stack_slice_count = win_count - self.master_count - 1
        else
            master_slice_count = win_count - 1
        end

        -- layout the master side
        if master_slice_count > 0 then
            local coord
            local len
            local axis

            if self.master_side == "left" or self.master_side == "right" then
                coord = master_rect.y
                len = master_rect.height
                axis = "horizontal"
            else
                coord = master_rect.x
                len = master_rect.width
                axis = "vertical"
            end

            for i = 1, master_slice_count do
                local slice_point = coord + math.floor(len * i + 0.5)
                slice_point = slice_point - gaps // 2
                local to_push, rest = master_rect:split_at(axis, slice_point, gaps)
                table.insert(geos, to_push)
                master_rect = rest
            end
        end

        table.insert(geos, master_rect)

        if stack_slice_count then
            assert(stack_rect)

            if stack_slice_count > 0 then
                local coord
                local len
                local axis
                if self.master_side == "left" or self.master_side == "right" then
                    coord = stack_rect.y
                    len = stack_rect.height / (stack_slice_count + 1)
                    axis = "horizontal"
                else
                    coord = stack_rect.x
                    len = stack_rect.width / (stack_slice_count + 1)
                    axis = "vertical"
                end

                for i = 1, stack_slice_count do
                    local slice_point = coord + math.floor(len * i + 0.5)
                    slice_point = slice_point - gaps // 2
                    local to_push, rest = stack_rect:split_at(axis, slice_point, gaps)
                    table.insert(geos, to_push)
                    stack_rect = rest
                end
            end

            table.insert(geos, stack_rect)
        end

        return geos
    else
        local origin_x = self.gaps.outer
        local origin_y = self.gaps.outer
        width = width - self.gaps.outer * 2
        height = height - self.gaps.outer * 2

        if win_count == 1 then
            table.insert(geos, {
                x = origin_x + self.gaps.inner,
                y = origin_y + self.gaps.inner,
                width = width - self.gaps.inner * 2,
                height = height - self.gaps.inner * 2,
            })
            return geos
        end

        local h = height / win_count
        local y_s = {}
        for i = 0, win_count - 1 do
            table.insert(y_s, math.floor(i * h + 0.5))
        end
        local heights = {}
        for i = 1, win_count - 1 do
            table.insert(heights, y_s[i + 1] - y_s[i])
        end
        table.insert(heights, height - y_s[win_count])

        for i = 1, win_count do
            table.insert(geos, { x = origin_x, y = origin_y + y_s[i], width = width, height = heights[i] })
        end

        for i = 1, #geos do
            geos[i].x = geos[i].x + self.gaps.inner
            geos[i].y = geos[i].y + self.gaps.inner
            geos[i].width = geos[i].width - self.gaps.inner * 2
            geos[i].height = geos[i].height - self.gaps.inner * 2
        end

        return geos
    end
end

---@class Layout
local layout = {
    builtins = builtins,
}

---@param handler LayoutHandler
function layout.set_handler(handler)
    client.bidirectional_streaming_request(
        build_grpc_request_params("Layout", {
            layout = {},
        }),
        function(response, stream)
            local request_id = response.request_id
            local index = handler.index

            ---@diagnostic disable-next-line: invisible
            local output_handle = require("pinnacle.output").handle.new(response.output_name)

            ---@diagnostic disable-next-line: invisible
            local window_handles = require("pinnacle.window").handle.new_from_table(response.window_ids or {})

            ---@diagnostic disable-next-line: invisible
            local tag_handles = require("pinnacle.tag").handle.new_from_table(response.tag_ids or {})

            ---@type LayoutArgs
            local args = {
                output = output_handle,
                windows = window_handles,
                tags = tag_handles,
                output_width = response.output_width,
                output_height = response.output_height,
            }

            local geos = handler.layouts[index]:layout(args)

            local body = protobuf.encode(".pinnacle.layout.v0alpha1.LayoutRequest", {
                geometries = {
                    request_id = request_id,
                    geometries = geos,
                    output_name = response.output_name,
                },
            })

            stream:write_chunk(body, false)
        end
    )
end

---@class LayoutHandlerModule
local layout_handler = {}

---@class LayoutHandler
---@field index integer
---@field layouts { layout: fun(self: self, args: LayoutArgs): { x: integer, y: integer, width: integer, height: integer }[] }[]
local LayoutHandler = {}

---@param layouts { layout: fun(self: self, args: LayoutArgs): { x: integer, y: integer, width: integer, height: integer }[] }[]
---@return LayoutHandler
function layout_handler.new(layouts)
    ---@type LayoutHandler
    local self = {
        index = 1,
        layouts = layouts,
    }

    setmetatable(self, { __index = LayoutHandler })

    return self
end

---@param layouts { layout: fun(self: self, args: LayoutArgs): { x: integer, y: integer, width: integer, height: integer }[] }[]
---
---@return LayoutHandler
function layout.new_handler(layouts)
    return layout_handler.new(layouts)
end

return layout
