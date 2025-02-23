pcall(require, "compat53")

local socket = require("cqueues.socket")
local headers = require("http.headers")
local h2_connection = require("http.h2_connection")
local pb = require("pb")

local grpc_client = {}

---@class grpc_client.Client
---@field conn grpc_client.h2.Conn
---@field loop grpc_client.cqueues.Loop
local Client = {}

---Create a new gRPC client that connects to the socket specified with `sock_args`.
---See `socket.connect` in the cqueues manual for more information.
---
---@nodiscard
---@param sock_args any A table of named arguments from `cqueues.socket.connect`
---@return grpc_client.Client
function grpc_client.new(sock_args)
    local sock = socket.connect(sock_args)
    sock:connect()

    local conn = h2_connection.new(sock, "client")
    conn:connect()

    ---@type grpc_client.Client
    local ret = {
        conn = conn,
        loop = require("cqueues").new(),
    }

    setmetatable(ret, { __index = Client })

    return ret
end

---Encodes the given `data` as the protobuf `type`.
---
---@param type string The absolute protobuf type
---@param data table The table of data, conforming to its protobuf definition
---@return string bytes The encoded bytes
local function encode(type, data)
    local success, obj = pcall(pb.encode, type, data)
    if not success then
        error("failed to encode `" .. type .. "`: " .. obj)
    end

    local encoded_protobuf = obj

    -- The packed flag; one byte, 0 if not packed, 1 if packed.
    local packed_prefix = string.pack("I1", 0)
    -- The payload length as a 4-byte big-endian integer
    local payload_len = string.pack(">I4", encoded_protobuf:len())

    local body = packed_prefix .. payload_len .. encoded_protobuf

    return body
end

---Creates headers for a gRPC request.
---
---@param service string The desired service
---@param method string The desired method within the service
local function create_request_headers(service, method)
    local req_headers = headers.new()
    req_headers:append(":method", "POST")
    req_headers:append(":scheme", "http")
    req_headers:append(":path", "/" .. service .. "/" .. method)
    req_headers:append("te", "trailers")
    req_headers:append("content-type", "application/grpc")
    return req_headers
end

---gRPC status codes taken from https://grpc.io/docs/guides/status-codes/
---
---@enum grpc_client.Status
local status = {
    ---Not an error; returned on success.
    OK = 0,
    ---The operation was cancelled, typically by the caller.
    CANCELLED = 1,
    ---Unknown error. For example, this error may be returned when a Status value
    ---received from another address space belongs to an error space that is not
    ---known in this address space. Also errors raised by APIs that do not return
    ---enough error information may be converted to this error.
    UNKNOWN = 2,
    ---The client specified an invalid argument. Note that this differs from FAILED_PRECONDITION.
    ---INVALID_ARGUMENT indicates arguments that are problematic regardless of the state
    ---of the system (e.g., a malformed file name).
    INVALID_ARGUMENT = 3,
    ---The deadline expired before the operation could complete. For operations
    ---that change the state of the system, this error may be returned even if
    ---the operation has completed successfully. For example, a successful response
    ---from a server could have been delayed long
    DEADLINE_EXCEEDED = 4,
    ---Some requested entity (e.g., file or directory) was not found.
    ---Note to server developers: if a request is denied for an entire class of users,
    ---such as gradual feature rollout or undocumented allowlist, NOT_FOUND may be used.
    ---If a request is denied for some users within a class of users,
    ---such as user-based access control, PERMISSION_DENIED must be used.
    NOT_FOUND = 5,
    ---The entity that a client attempted to create (e.g., file or directory) already exists.
    ALREADY_EXISTS = 6,
    ---The caller does not have permission to execute the specified operation.
    ---PERMISSION_DENIED must not be used for rejections caused by exhausting some resource
    ---(use RESOURCE_EXHAUSTED instead for those errors). PERMISSION_DENIED must not be used
    ---if the caller can not be identified (use UNAUTHENTICATED instead for those errors).
    ---This error code does not imply the request is valid or the requested entity exists
    ---or satisfies other pre-conditions.
    PERMISSION_DENIED = 7,
    ---Some resource has been exhausted, perhaps a per-user quota,
    ---or perhaps the entire file system is out of space.
    RESOURCE_EXHAUSTED = 8,
    ---The operation was rejected because the system is not in a state required for
    ---the operation’s execution. For example, the directory to be deleted is non-empty,
    ---an rmdir operation is applied to a non-directory, etc. Service implementors can use
    ---the following guidelines to decide between FAILED_PRECONDITION, ABORTED, and UNAVAILABLE:
    ---(a) Use UNAVAILABLE if the client can retry just the failing call.
    ---(b) Use ABORTED if the client should retry at a higher level
    ---(e.g., when a client-specified test-and-set fails, indicating the
    ---client should restart a read-modify-write sequence).
    ---(c) Use FAILED_PRECONDITION if the client should not retry until the system state
    ---has been explicitly fixed. E.g., if an “rmdir” fails because the directory is non-empty,
    ---FAILED_PRECONDITION should be returned since the client should not retry unless
    ---the files are deleted from the directory.
    FAILED_PRECONDITION = 9,
    ---The operation was aborted, typically due to a concurrency issue such as
    ---a sequencer check failure or transaction abort. See the guidelines above for
    ---deciding between FAILED_PRECONDITION, ABORTED, and UNAVAILABLE.
    ABORTED = 10,
    ---The operation was attempted past the valid range. E.g., seeking or reading
    ---past end-of-file. Unlike INVALID_ARGUMENT, this error indicates a problem
    ---that may be fixed if the system state changes. For example, a 32-bit file system
    ---will generate INVALID_ARGUMENT if asked to read at an offset that is not
    ---in the range [0,2^32-1], but it will generate OUT_OF_RANGE if asked to read
    ---from an offset past the current file size. There is a fair bit of overlap between
    ---FAILED_PRECONDITION and OUT_OF_RANGE. We recommend using OUT_OF_RANGE
    ---(the more specific error) when it applies so that callers who are iterating
    ---through a space can easily look for an OUT_OF_RANGE error to detect when they are done.
    OUT_OF_RANGE = 11,
    ---The operation is not implemented or is not supported/enabled in this service.
    UNIMPLEMENTED = 12,
    ---Internal errors. This means that some invariants expected by the underlying system
    ---have been broken. This error code is reserved for serious errors.
    INTERNAL = 13,
    ---The service is currently unavailable. This is most likely a transient condition,
    ---which can be corrected by retrying with a backoff.
    ---Note that it is not always safe to retry non-idempotent operations.
    UNAVAILABLE = 14,
    ---Unrecoverable data loss or corruption.
    DATA_LOSS = 15,
    ---The request does not have valid authentication credentials for the operation.
    UNAUTHENTICATED = 16,
}

local code_to_status = {
    [0] = "OK",
    [1] = "CANCELLED",
    [2] = "UNKNOWN",
    [3] = "INVALID_ARGUMENT",
    [4] = "DEADLINE_EXCEEDED",
    [5] = "NOT_FOUND",
    [6] = "ALREADY_EXISTS",
    [7] = "PERMISSION_DENIED",
    [8] = "RESOURCE_EXHAUSTED",
    [9] = "FAILED_PRECONDITION",
    [10] = "ABORTED",
    [11] = "OUT_OF_RANGE",
    [12] = "UNIMPLEMENTED",
    [13] = "INTERNAL",
    [14] = "UNAVAILABLE",
    [15] = "DATA_LOSS",
    [16] = "UNAUTHENTICATED",
}

---Retrive the name for a gRPC status code, or `nil` if the given
---`code` does not correspond to one.
---
---@param code integer
---
---@return string|nil
function status.name(code)
    return code_to_status[code]
end

---Perform a unary request.
---
---@nodiscard
---
---@param request_specifier grpc_client.RequestSpecifier
---@param data table The message to send. This should be in the structure of `request_specifier.request`.
---
---@return table|nil response The response as a table in the structure of `request_specifier.response`, or `nil` if there is an error.
---@return string|nil error An error string, if any.
function Client:unary_request(request_specifier, data)
    local stream = self.conn:new_stream()

    local service = request_specifier.service
    local method = request_specifier.method
    local request_type = request_specifier.request
    local response_type = request_specifier.response

    local body = encode(request_type, data)

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    local headers = stream:get_headers()
    local grpc_status = headers:get("grpc-status")
    if grpc_status then
        local grpc_status = tonumber(grpc_status)
        if grpc_status ~= 0 then
            local err_name = status.name(grpc_status)
            local grpc_msg = headers:get("grpc-message")
            local grpc_msg = grpc_msg and (", msg = " .. grpc_msg) or ""
            local err_str = "error from response: code = "
                .. (err_name or "unknown grpc status code")
                .. grpc_msg
            return nil, err_str
        end
    end

    local response_body = stream:get_next_chunk()

    local trailers = stream:get_headers()
    if trailers then -- idk if im big dummy or not but there are never any trailers
        for name, value, never_index in trailers:each() do
            print(name, value, never_index)
        end
    end

    stream:shutdown()

    -- string:sub(6) to skip the 1-byte compressed flag and the 4-byte message length
    local response = pb.decode(response_type, response_body:sub(6))

    return response, nil
end

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param request_specifier grpc_client.RequestSpecifier
---@param data table The message to send. This should be in the structure of `request_specifier.request`.
---@param callback fun(response: table) A callback that will be run with every response
---
---@return string|nil error An error string, if any.
function Client:server_streaming_request(request_specifier, data, callback)
    local stream = self.conn:new_stream()

    local service = request_specifier.service
    local method = request_specifier.method
    local request_type = request_specifier.request
    local response_type = request_specifier.response

    local body = encode(request_type, data)

    stream:write_headers(create_request_headers(service, method), false)
    stream:write_chunk(body, true)

    local headers = stream:get_headers()
    local grpc_status = headers:get("grpc-status")
    if grpc_status then
        local grpc_status = tonumber(grpc_status)
        if grpc_status ~= 0 then
            local err_name = status.name(grpc_status)
            local err_str = "error from response: " .. (err_name or "unknown grpc status code")
            return err_str
        end
    end

    self.loop:wrap(function()
        for response_body in stream:each_chunk() do
            while response_body:len() > 0 do
                local msg_len = string.unpack(">I4", response_body:sub(2, 5))

                -- Skip the 1-byte compressed flag and the 4-byte message length
                local body = response_body:sub(6, 6 + msg_len - 1)

                ---@diagnostic disable-next-line: redefined-local
                local success, obj = pcall(pb.decode, response_type, body)
                if not success then
                    print(obj)
                    os.exit(1)
                end

                local response = obj
                callback(response)

                response_body = response_body:sub(msg_len + 6)
            end
        end

        local trailers = stream:get_headers()
        if trailers then
            for name, value, never_index in trailers:each() do
                print(name, value, never_index)
            end
        end
    end)

    return nil
end

---Performs a bidirectional-streaming request.
---
---`callback` will be called with every streamed response.
---
---The raw client-to-server stream is returned to allow you to send encoded messages.
---
---@nodiscard
---
---@param request_specifier grpc_client.RequestSpecifier
---@param callback fun(response: table, stream: grpc_client.h2.Stream) A callback that will be run with every response
---
---@return grpc_client.h2.Stream|nil
---@return string|nil error An error string, if any.
function Client:bidirectional_streaming_request(request_specifier, callback)
    local stream = self.conn:new_stream()

    local service = request_specifier.service
    local method = request_specifier.method
    local response_type = request_specifier.response

    stream:write_headers(create_request_headers(service, method), false)

    local headers = stream:get_headers()
    local grpc_status = headers:get("grpc-status")
    if grpc_status then
        local grpc_status = tonumber(grpc_status)
        if grpc_status ~= 0 then
            local err_name = status.name(grpc_status)
            local err_str = "error from response: " .. (err_name or "unknown grpc status code")
            return nil, err_str
        end
    end

    self.loop:wrap(function()
        for response_body in stream:each_chunk() do
            while response_body:len() > 0 do
                local msg_len = string.unpack(">I4", response_body:sub(2, 5))

                -- Skip the 1-byte compressed flag and the 4-byte message length
                local body = response_body:sub(6, 6 + msg_len - 1)

                ---@diagnostic disable-next-line: redefined-local
                local success, obj = pcall(pb.decode, response_type, body)
                if not success then
                    print(obj)
                    os.exit(1)
                end

                local response = obj
                callback(response, stream)

                response_body = response_body:sub(msg_len + 6)
            end
        end

        local trailers = stream:get_headers()
        if trailers then
            for name, value, never_index in trailers:each() do
                print(name, value, never_index)
            end
        end
    end)

    return stream, nil
end

-- Definitions

---@class grpc_client.h2.Conn
---@field new_stream fun(self: self): grpc_client.h2.Stream
---@field ping fun(self: self, timeout_secs: integer)

---@class grpc_client.cqueues.Loop
---@field loop function
---@field wrap fun(self: self, fn: function)

---@class grpc_client.h2.Stream
---@field write_chunk function
---@field shutdown function
---@field write_headers function
---@field get_headers function
---@field get_next_chunk function
---@field each_chunk function

---@class grpc_client.RequestSpecifier
---@field service string The fully-qualified service name
---@field method string The method name
---@field request string The fully-qualified request type
---@field response string The fully-qualified response type
