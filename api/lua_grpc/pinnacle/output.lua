---The protobuf absolute path prefix
local prefix = "pinnacle.output." .. require("pinnacle").version .. "."
local service = prefix .. "OutputService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) OutputServiceMethod
local rpc_types = {
    SetLocation = {},
    ConnectForAll = {
        response_type = "ConnectForAllResponse",
    },
    Get = {
        response_type = "GetResponse",
    },
    GetProperties = {
        response_type = "GetPropertiesResponse",
    },
}

---Build GrpcRequestParams
---@param method OutputServiceMethod
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

---@class OutputHandleModule
local output_handle = {}

---@class OutputHandle
---@field private config_client Client
---@field name string The unique name of this output
local OutputHandle = {}

---@class OutputModule
---@field private handle OutputHandleModule
local output = {}
output.handle = output_handle

---@class Output
---@field private config_client Client
local Output = {}

---Get all outputs.
---
---@return OutputHandle[]
function Output:get_all()
    local response = self.config_client:unary_request(build_grpc_request_params("Get", {}))

    ---@type OutputHandle[]
    local handles = {}

    for _, output_name in pairs(response.output_names) do
        table.insert(handles, output_handle.new(self.config_client, output_name))
    end

    return handles
end

---@param name string The name of the port the output is connected to
---@return OutputHandle | nil
function Output:get_by_name(name)
    local handles = self:get_all()

    for _, handle in pairs(handles) do
        if handle.name == name then
            return handle
        end
    end

    return nil
end

---@return OutputHandle | nil
function Output:get_focused()
    local handles = self:get_all()

    for _, handle in pairs(handles) do
        if handle:props().focused then
            return handle
        end
    end

    return nil
end

---@param callback fun(output: OutputHandle)
function Output:connect_for_all(callback)
    self.config_client:server_streaming_request(build_grpc_request_params("ConnectForAll", {}), function(response)
        local output_name = response.output_name
        local handle = output_handle.new(self.config_client, output_name)
        callback(handle)
    end)
end

---@param loc { x: integer?, y: integer? }
function OutputHandle:set_location(loc)
    self.config_client:unary_request(build_grpc_request_params("SetLocation", {
        output_name = self.name,
        x = loc.x,
        y = loc.y,
    }))
end

---@class OutputProperties
---@field make string?
---@field model string?
---@field x integer?
---@field y integer?
---@field pixel_width integer?
---@field pixel_height integer?
---@field refresh_rate integer?
---@field physical_width integer?
---@field physical_height integer?
---@field focused boolean?
---@field tags TagHandle[]

---Get all properties of this output.
---@return OutputProperties
function OutputHandle:props()
    local response =
        self.config_client:unary_request(build_grpc_request_params("GetProperties", { output_name = self.name }))

    local handles = require("pinnacle.tag").handle.new_from_table(self.config_client, response.tag_ids)

    response.tags = handles
    response.tag_ids = nil

    return response
end

---@return Output
function output.new(config_client)
    ---@type Output
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Output })
    return self
end

---Create a new `OutputHandle` from its raw name.
---@param output_name string
function output_handle.new(config_client, output_name)
    ---@type OutputHandle
    local self = {
        config_client = config_client,
        name = output_name,
    }
    setmetatable(self, { __index = OutputHandle })
    return self
end

return output
