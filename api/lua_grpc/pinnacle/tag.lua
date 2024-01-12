---The protobuf absolute path prefix
local prefix = "pinnacle.tag." .. require("pinnacle").version .. "."
local service = prefix .. "TagService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) TagServiceMethod
local rpc_types = {
    SetActive = {},
    SwitchTo = {},
    Add = {
        response_type = "AddResponse",
    },
    Remove = {},
    SetLayout = {},
    Get = {
        response_type = "GetResponse",
    },
    GetProperties = {
        response_type = "GetPropertiesResponse",
    },
}

---Build GrpcRequestParams
---@param method TagServiceMethod
---@param data table
---@return GrpcRequestParams
local function build_grpc_request_params(method, data)
    local req_type = rpc_types[method].request_type
    local resp_type = rpc_types[method].response_type

    ---@type GrpcRequestParams
    return {
        service = service,
        method = method,
        request_type = req_type and prefix .. req_type,
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---@class TagHandleModule
local tag_handle = {}

---@class TagHandle
---@field private config_client Client
---@field id integer
local TagHandle = {}

---@class TagModule
---@field private handle TagHandleModule
local tag = {}
tag.handle = tag_handle

---@class Tag
---@field private config_client Client
local Tag = {}

---Get all tags across all outputs.
---
---@return TagHandle[]
function Tag:get_all()
    local response = self.config_client:unary_request(build_grpc_request_params("Get", {}))

    ---@type TagHandle[]
    local handles = {}

    for _, id in pairs(response.tag_ids) do
        table.insert(handles, tag_handle.new(self.config_client, id))
    end

    return handles
end

---Add tags with the given names to the specified output.
---
---Returns handles to the created tags.
---
---@param output OutputHandle
---@param ... string
---
---@return TagHandle[]
---
---@overload fun(output: OutputHandle, tag_names: string[])
function Tag:add(output, ...)
    local tag_names = { ... }
    if type(tag_names[1]) == "table" then
        tag_names = tag_names[1] --[=[@as string[]]=]
    end

    local response = self.config_client:unary_request(build_grpc_request_params("Add", {
        output_name = output.name,
        tag_names = tag_names,
    }))

    ---@type TagHandle[]
    local handles = {}

    for _, id in pairs(response.tag_ids) do
        table.insert(handles, tag_handle.new(self.config_client, id))
    end

    return handles
end

---Remove the given tags.
---
---@param tags TagHandle[]
function Tag:remove(tags)
    ---@type integer[]
    local ids = {}

    for _, tg in pairs(tags) do
        table.insert(ids, tg.id)
    end

    self.config_client:unary_request(build_grpc_request_params("Remove", { tag_ids = ids }))
end

---@class LayoutCycler
---@field next fun(output: OutputHandle)
---@field prev fun(output: OutputHandle)

--- TODO: docs
---@param layouts Layout[]
---
---@return LayoutCycler
function Tag:new_layout_cycler(layouts)
    local indices = {}

    if #layouts == 0 then
        return {
            next = function(_) end,
            prev = function(_) end,
        }
    end

    ---@type LayoutCycler
    return {
        next = function(output)
            local tags = output:props().tags

            for _, tg in pairs(tags) do
                if tg:props().active then
                    local id = tg.id
                    if #layouts == 1 then
                        indices[id] = 1
                    elseif indices[id] == nil then
                        indices[id] = 2
                    else
                        if indices[id] + 1 > #layouts then
                            indices[id] = 1
                        else
                            indices[id] = indices[id] + 1
                        end
                    end

                    tg:set_layout(layouts[indices[id]])
                    break
                end
            end
        end,
        prev = function(output)
            local tags = output:props().tags

            for _, tg in pairs(tags) do
                if tg:props().active then
                    local id = tg.id

                    if #layouts == 1 then
                        indices[id] = 1
                    elseif indices[id] == nil then
                        indices[id] = #layouts - 1
                    else
                        if indices[id] - 1 < 1 then
                            indices[id] = #layouts
                        else
                            indices[id] = indices[id] - 1
                        end
                    end

                    tg:set_layout(layouts[indices[id]])
                    break
                end
            end
        end,
    }
end

---Remove this tag.
function TagHandle:remove()
    self.config_client:unary_request(build_grpc_request_params("Remove", { tag_ids = { self.id } }))
end

---@enum (key) Layout
local _layouts = {
    master_stack = 1,
    dwindle = 2,
    spiral = 3,
    corner_top_left = 4,
    corner_top_right = 5,
    corner_bottom_left = 6,
    corner_bottom_right = 7,
}

---@param layout Layout
function TagHandle:set_layout(layout)
    local layout = _layouts[layout]

    self.config_client:unary_request(build_grpc_request_params("SetLayout", {
        tag_id = self.id,
        layout = layout,
    }))
end

---Activate this tag and deactivate all other ones on the same output.
function TagHandle:switch_to()
    self.config_client:unary_request(build_grpc_request_params("SwitchTo", { tag_id = self.id }))
end

---Set whether or not this tag is active.
---
---@param active boolean
function TagHandle:set_active(active)
    self.config_client:unary_request(build_grpc_request_params("SetActive", { tag_id = self.id, set = active }))
end

---Toggle this tag's active state.
function TagHandle:toggle_active()
    self.config_client:unary_request(build_grpc_request_params("SetActive", { tag_id = self.id, toggle = {} }))
end

---@class TagProperties
---@field active boolean?
---@field name string?
---@field output OutputHandle?

---Get all properties of this tag.
---
---@return TagProperties
function TagHandle:props()
    local response = self.config_client:unary_request(build_grpc_request_params("GetProperties", { tag_id = self.id }))

    return {
        active = response.active,
        name = response.name,
        output = response.output_name
            and require("pinnacle.output").handle.new(self.config_client, response.output_name),
    }
end

---@return Tag
function tag.new(config_client)
    ---@type Tag
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Tag })
    return self
end

---Create a new `TagHandle` from an id.
---@param config_client Client
---@param tag_id integer
---@return TagHandle
function tag_handle.new(config_client, tag_id)
    ---@type TagHandle
    local self = {
        config_client = config_client,
        id = tag_id,
    }
    setmetatable(self, { __index = TagHandle })
    return self
end

---@param config_client Client
---@param tag_ids integer[]
---@return TagHandle[]
function tag_handle.new_from_table(config_client, tag_ids)
    ---@type TagHandle[]
    local handles = {}

    for _, id in pairs(tag_ids) do
        table.insert(handles, tag_handle.new(config_client, id))
    end

    return handles
end

return tag
