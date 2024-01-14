local socket = require("cqueues.socket")
local headers = require("http.headers")
local h2_connection = require("http.h2_connection")
local pb = require("pb")
local inspect = require("inspect")

---Create appropriate headers for a gRPC request.
---@param service string The desired service
---@param method string The desired method within the service
---@return HttpHeaders
local function create_request_headers(service, method)
    local req_headers = headers.new()
    req_headers:append(":method", "POST")
    req_headers:append(":scheme", "http")
    req_headers:append(":path", "/" .. service .. "/" .. method)
    req_headers:append("te", "trailers")
    req_headers:append("content-type", "application/grpc")
    return req_headers
end

---@param body string
---@return boolean compressed
---@return integer length
---@return string message
---@return integer total_length
local function read_length_prefixed_message(body) end

---@class ClientModule
local client = {}

---@class Client
---@field conn H2Connection
---@field loop CqueuesLoop
local Client = {}

---@class GrpcRequestParams
---@field service string
---@field method string
---@field request_type string
---@field response_type string?
---@field data table

---Send a synchronous unary request to the compositor.
---
---If `request_type` is not specified then it will default to
---`method` .. "Request".
---
---If `response_type` is not specified then it will default to
---`google.protobuf.Empty`.
---@param grpc_request_params GrpcRequestParams
---@return table
function Client:unary_request(grpc_request_params)
    local stream = self.conn:new_stream()

    local service = grpc_request_params.service
    local method = grpc_request_params.method
    local request_type = grpc_request_params.request_type
    local response_type = grpc_request_params.response_type or "google.protobuf.Empty"
    local data = grpc_request_params.data

    local encoded_protobuf = assert(pb.encode(request_type, data), "wrong table schema")

    local packed_prefix = string.pack("I1", 0)
    local payload_len = string.pack(">I4", encoded_protobuf:len())

    local body = packed_prefix .. payload_len .. encoded_protobuf

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    -- for chunk in stream:each_chunk() do
    --     print(chunk, ":", pb.tohex(chunk))
    --     os.exit(1)
    -- end

    local response_headers = stream:get_headers()
    -- TODO: check headers for errors

    local response_body = stream:get_next_chunk()
    print("unary body", response_body, "end")
    print(pb.tohex(response_body))
    print("--------------------------------")

    local trailers = stream:get_headers()
    print("trailers", trailers)
    print("------------------------------")
    if trailers then
        for name, value, never_index in trailers:each() do
            print(name, value, never_index)
        end
    end

    -- stream:shutdown()

    -- Skip the 1-byte compressed flag and the 4-byte message length
    local response_body = response_body:sub(6)
    local response = pb.decode(response_type, response_body)

    return response
end

---Send a async server streaming request to the compositor.
---
---`callback` will be called with every streamed response.
---
---If `response_type` is not specified then it will default to
---`google.protobuf.Empty`.
---@param grpc_request_params GrpcRequestParams
---@param callback fun(response: table)
function Client:server_streaming_request(grpc_request_params, callback)
    -- print(inspect(grpc_request_params))
    local stream = self.conn:new_stream()

    local service = grpc_request_params.service
    local method = grpc_request_params.method
    local request_type = grpc_request_params.request_type
    local response_type = grpc_request_params.response_type or "google.protobuf.Empty"
    local data = grpc_request_params.data

    local success, obj = pcall(pb.encode, request_type, data)
    if not success then
        print("failed to encode:", obj, "for", service, method, request_type, response_type)
        os.exit(1)
    end

    local encoded_protobuf = obj

    local packed_prefix = string.pack("I1", 0)
    local payload_len = string.pack(">I4", encoded_protobuf:len())

    local body = packed_prefix .. payload_len .. encoded_protobuf

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    local response_headers = stream:get_headers()
    for name, value, never_index in response_headers:each() do
        print(name, value, never_index)
    end
    -- local chunk = stream:get_next_chunk()
    -- print(chunk, chunk:len())
    -- TODO: check headers for errors

    self.loop:wrap(function()
        for response_body in stream:each_chunk() do
            print("stream chunk", response_body, "end")
            print(pb.tohex(response_body))
            print("-----------------------------------")
            -- Skip the 1-byte compressed flag and the 4-byte message length
            local response_body = response_body:sub(6)

            local success, obj = pcall(pb.decode, response_type, response_body)
            if not success then
                print(obj)
                os.exit(1)
            end

            local response = obj
            callback(response)
        end

        local trailers = stream:get_headers()
        if trailers then
            for name, value, never_index in trailers:each() do
                print(name, value, never_index)
            end
        end
    end)
end

---@return Client
function client.new(loop)
    local sock = socket.connect({
        -- host = "127.0.0.1",
        -- port = "8080",
        path = "/tmp/pinnacle/grpc.sock",
    })
    sock:connect()

    local conn = h2_connection.new(sock, "client")
    conn:connect()

    ---@type Client
    local self = {
        conn = conn,
        loop = loop,
    }
    setmetatable(self, { __index = Client })

    return self
end

return client
