---The protobuf absolute path prefix
local prefix = "pinnacle.process." .. require("pinnacle").version .. "."
local service = prefix .. "ProcessService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) ProcessServiceMethod
local rpc_types = {
    Spawn = {
        response_type = "SpawnResponse",
    },
    SetEnv = {},
}

---Build GrpcRequestParams
---@param method ProcessServiceMethod
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

---@class ProcessModule
local process = {}

---@class Process
---@field private config_client Client
local Process = {}

---@param args string[]
---@param callbacks { stdout: fun(line: string)?, stderr: fun(line: string)?, exit: fun(code: integer, msg: string)? }?
function Process:spawn(args, callbacks)
    local callback = function() end

    if callbacks then
        callback = function(response)
            if callbacks.stdout and response.stdout then
                callbacks.stdout(response.stdout)
            end
            if callbacks.stderr and response.stderr then
                callbacks.stderr(response.stderr)
            end
            if callbacks.exit and (response.exit_code or response.exit_message) then
                callbacks.exit(response.exit_code, response.exit_message)
            end
        end
    end

    self.config_client:server_streaming_request(
        build_grpc_request_params("Spawn", {
            args = args,
            once = false,
            has_callback = callbacks ~= nil,
        }),
        callback
    )
end

function process.new(config_client)
    ---@type Process
    local self = { config_client = config_client }
    setmetatable(self, { __index = Process })
    return self
end

return process
