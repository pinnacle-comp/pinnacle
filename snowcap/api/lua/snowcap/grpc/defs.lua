---@lcat nodoc

---@lcat nodoc

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

---@enum snowcap.widget.v0alpha1.Alignment
local snowcap_widget_v0alpha1_Alignment = {
    ALIGNMENT_UNSPECIFIED = 0,
    ALIGNMENT_START = 1,
    ALIGNMENT_CENTER = 2,
    ALIGNMENT_END = 3,
}

---@enum snowcap.widget.v0alpha1.ScrollableAlignment
local snowcap_widget_v0alpha1_ScrollableAlignment = {
    SCROLLABLE_ALIGNMENT_UNSPECIFIED = 0,
    SCROLLABLE_ALIGNMENT_START = 1,
    SCROLLABLE_ALIGNMENT_END = 2,
}

---@enum snowcap.widget.v0alpha1.Font.Weight
local snowcap_widget_v0alpha1_Font_Weight = {
    WEIGHT_UNSPECIFIED = 0,
    WEIGHT_THIN = 1,
    WEIGHT_EXTRA_LIGHT = 2,
    WEIGHT_LIGHT = 3,
    WEIGHT_NORMAL = 4,
    WEIGHT_MEDIUM = 5,
    WEIGHT_SEMIBOLD = 6,
    WEIGHT_BOLD = 7,
    WEIGHT_EXTRA_BOLD = 8,
    WEIGHT_BLACK = 9,
}

---@enum snowcap.widget.v0alpha1.Font.Stretch
local snowcap_widget_v0alpha1_Font_Stretch = {
    STRETCH_UNSPECIFIED = 0,
    STRETCH_ULTRA_CONDENSED = 1,
    STRETCH_EXTRA_CONDENSED = 2,
    STRETCH_CONDENSED = 3,
    STRETCH_SEMI_CONDENSED = 4,
    STRETCH_NORMAL = 5,
    STRETCH_SEMI_EXPANDED = 6,
    STRETCH_EXPANDED = 7,
    STRETCH_EXTRA_EXPANDED = 8,
    STRETCH_ULTRA_EXPANDED = 9,
}

---@enum snowcap.widget.v0alpha1.Font.Style
local snowcap_widget_v0alpha1_Font_Style = {
    STYLE_UNSPECIFIED = 0,
    STYLE_NORMAL = 1,
    STYLE_ITALIC = 2,
    STYLE_OBLIQUE = 3,
}

---@enum snowcap.layer.v0alpha1.Anchor
local snowcap_layer_v0alpha1_Anchor = {
    ANCHOR_UNSPECIFIED = 0,
    ANCHOR_TOP = 1,
    ANCHOR_BOTTOM = 2,
    ANCHOR_LEFT = 3,
    ANCHOR_RIGHT = 4,
    ANCHOR_TOP_LEFT = 5,
    ANCHOR_TOP_RIGHT = 6,
    ANCHOR_BOTTOM_LEFT = 7,
    ANCHOR_BOTTOM_RIGHT = 8,
}

---@enum snowcap.layer.v0alpha1.KeyboardInteractivity
local snowcap_layer_v0alpha1_KeyboardInteractivity = {
    KEYBOARD_INTERACTIVITY_UNSPECIFIED = 0,
    KEYBOARD_INTERACTIVITY_NONE = 1,
    KEYBOARD_INTERACTIVITY_ON_DEMAND = 2,
    KEYBOARD_INTERACTIVITY_EXCLUSIVE = 3,
}

---@enum snowcap.layer.v0alpha1.Layer
local snowcap_layer_v0alpha1_Layer = {
    LAYER_UNSPECIFIED = 0,
    LAYER_BACKGROUND = 1,
    LAYER_BOTTOM = 2,
    LAYER_TOP = 3,
    LAYER_OVERLAY = 4,
}

---@enum snowcap.widget.v1.Alignment
local snowcap_widget_v1_Alignment = {
    ALIGNMENT_UNSPECIFIED = 0,
    ALIGNMENT_START = 1,
    ALIGNMENT_CENTER = 2,
    ALIGNMENT_END = 3,
}

---@enum snowcap.widget.v1.Font.Weight
local snowcap_widget_v1_Font_Weight = {
    WEIGHT_UNSPECIFIED = 0,
    WEIGHT_THIN = 1,
    WEIGHT_EXTRA_LIGHT = 2,
    WEIGHT_LIGHT = 3,
    WEIGHT_NORMAL = 4,
    WEIGHT_MEDIUM = 5,
    WEIGHT_SEMIBOLD = 6,
    WEIGHT_BOLD = 7,
    WEIGHT_EXTRA_BOLD = 8,
    WEIGHT_BLACK = 9,
}

---@enum snowcap.widget.v1.Font.Stretch
local snowcap_widget_v1_Font_Stretch = {
    STRETCH_UNSPECIFIED = 0,
    STRETCH_ULTRA_CONDENSED = 1,
    STRETCH_EXTRA_CONDENSED = 2,
    STRETCH_CONDENSED = 3,
    STRETCH_SEMI_CONDENSED = 4,
    STRETCH_NORMAL = 5,
    STRETCH_SEMI_EXPANDED = 6,
    STRETCH_EXPANDED = 7,
    STRETCH_EXTRA_EXPANDED = 8,
    STRETCH_ULTRA_EXPANDED = 9,
}

---@enum snowcap.widget.v1.Font.Style
local snowcap_widget_v1_Font_Style = {
    STYLE_UNSPECIFIED = 0,
    STYLE_NORMAL = 1,
    STYLE_ITALIC = 2,
    STYLE_OBLIQUE = 3,
}

---@enum snowcap.layer.v1.Anchor
local snowcap_layer_v1_Anchor = {
    ANCHOR_UNSPECIFIED = 0,
    ANCHOR_TOP = 1,
    ANCHOR_BOTTOM = 2,
    ANCHOR_LEFT = 3,
    ANCHOR_RIGHT = 4,
    ANCHOR_TOP_LEFT = 5,
    ANCHOR_TOP_RIGHT = 6,
    ANCHOR_BOTTOM_LEFT = 7,
    ANCHOR_BOTTOM_RIGHT = 8,
}

---@enum snowcap.layer.v1.KeyboardInteractivity
local snowcap_layer_v1_KeyboardInteractivity = {
    KEYBOARD_INTERACTIVITY_UNSPECIFIED = 0,
    KEYBOARD_INTERACTIVITY_NONE = 1,
    KEYBOARD_INTERACTIVITY_ON_DEMAND = 2,
    KEYBOARD_INTERACTIVITY_EXCLUSIVE = 3,
}

---@enum snowcap.layer.v1.Layer
local snowcap_layer_v1_Layer = {
    LAYER_UNSPECIFIED = 0,
    LAYER_BACKGROUND = 1,
    LAYER_BOTTOM = 2,
    LAYER_TOP = 3,
    LAYER_OVERLAY = 4,
}


---@alias google.protobuf.Empty nil

---@class snowcap.input.v0alpha1.Modifiers
---@field shift boolean?
---@field ctrl boolean?
---@field alt boolean?
---@field super boolean?

---@class snowcap.input.v0alpha1.KeyboardKeyRequest
---@field id integer?

---@class snowcap.input.v0alpha1.KeyboardKeyResponse
---@field key integer?
---@field modifiers snowcap.input.v0alpha1.Modifiers?
---@field pressed boolean?

---@class snowcap.input.v0alpha1.PointerButtonRequest
---@field id integer?

---@class snowcap.input.v0alpha1.PointerButtonResponse
---@field button integer?
---@field pressed boolean?

---@class snowcap.input.v1.Modifiers
---@field shift boolean?
---@field ctrl boolean?
---@field alt boolean?
---@field super boolean?

---@class snowcap.input.v1.KeyboardKeyRequest
---@field id integer?

---@class snowcap.input.v1.KeyboardKeyResponse
---@field key integer?
---@field modifiers snowcap.input.v1.Modifiers?
---@field pressed boolean?

---@class snowcap.input.v1.PointerButtonRequest
---@field id integer?

---@class snowcap.input.v1.PointerButtonResponse
---@field button integer?
---@field pressed boolean?

---@class snowcap.widget.v0alpha1.Padding
---@field top number?
---@field right number?
---@field bottom number?
---@field left number?

---@class snowcap.widget.v0alpha1.Length
---@field fill google.protobuf.Empty?
---@field fill_portion integer?
---@field shrink google.protobuf.Empty?
---@field fixed number?

---@class snowcap.widget.v0alpha1.Color
---@field red number?
---@field green number?
---@field blue number?
---@field alpha number?

---@class snowcap.widget.v0alpha1.Font
---@field family snowcap.widget.v0alpha1.Font.Family?
---@field weight snowcap.widget.v0alpha1.Font.Weight?
---@field stretch snowcap.widget.v0alpha1.Font.Stretch?
---@field style snowcap.widget.v0alpha1.Font.Style?

---@class snowcap.widget.v0alpha1.Font.Family
---@field name string?
---@field serif google.protobuf.Empty?
---@field sans_serif google.protobuf.Empty?
---@field cursive google.protobuf.Empty?
---@field fantasy google.protobuf.Empty?
---@field monospace google.protobuf.Empty?

---@class snowcap.widget.v0alpha1.WidgetDef
---@field text snowcap.widget.v0alpha1.Text?
---@field column snowcap.widget.v0alpha1.Column?
---@field row snowcap.widget.v0alpha1.Row?
---@field scrollable snowcap.widget.v0alpha1.Scrollable?
---@field container snowcap.widget.v0alpha1.Container?

---@class snowcap.widget.v0alpha1.Text
---@field text string?
---@field pixels number?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field horizontal_alignment snowcap.widget.v0alpha1.Alignment?
---@field vertical_alignment snowcap.widget.v0alpha1.Alignment?
---@field color snowcap.widget.v0alpha1.Color?
---@field font snowcap.widget.v0alpha1.Font?

---@class snowcap.widget.v0alpha1.Column
---@field spacing number?
---@field padding snowcap.widget.v0alpha1.Padding?
---@field item_alignment snowcap.widget.v0alpha1.Alignment?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field max_width number?
---@field clip boolean?
---@field children snowcap.widget.v0alpha1.WidgetDef[]?

---@class snowcap.widget.v0alpha1.Row
---@field spacing number?
---@field padding snowcap.widget.v0alpha1.Padding?
---@field item_alignment snowcap.widget.v0alpha1.Alignment?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field clip boolean?
---@field children snowcap.widget.v0alpha1.WidgetDef[]?

---@class snowcap.widget.v0alpha1.ScrollableDirection
---@field vertical snowcap.widget.v0alpha1.ScrollableProperties?
---@field horizontal snowcap.widget.v0alpha1.ScrollableProperties?

---@class snowcap.widget.v0alpha1.ScrollableProperties
---@field width number?
---@field margin number?
---@field scroller_width number?
---@field alignment snowcap.widget.v0alpha1.ScrollableAlignment?

---@class snowcap.widget.v0alpha1.Scrollable
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field direction snowcap.widget.v0alpha1.ScrollableDirection?
---@field child snowcap.widget.v0alpha1.WidgetDef?

---@class snowcap.widget.v0alpha1.Container
---@field padding snowcap.widget.v0alpha1.Padding?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field max_width number?
---@field max_height number?
---@field horizontal_alignment snowcap.widget.v0alpha1.Alignment?
---@field vertical_alignment snowcap.widget.v0alpha1.Alignment?
---@field clip boolean?
---@field child snowcap.widget.v0alpha1.WidgetDef?
---@field text_color snowcap.widget.v0alpha1.Color?
---@field background_color snowcap.widget.v0alpha1.Color?
---@field border_radius number?
---@field border_thickness number?
---@field border_color snowcap.widget.v0alpha1.Color?

---@class snowcap.layer.v0alpha1.NewLayerRequest
---@field widget_def snowcap.widget.v0alpha1.WidgetDef?
---@field width integer?
---@field height integer?
---@field anchor snowcap.layer.v0alpha1.Anchor?
---@field keyboard_interactivity snowcap.layer.v0alpha1.KeyboardInteractivity?
---@field exclusive_zone integer?
---@field layer snowcap.layer.v0alpha1.Layer?

---@class snowcap.layer.v0alpha1.NewLayerResponse
---@field layer_id integer?

---@class snowcap.layer.v0alpha1.CloseRequest
---@field layer_id integer?

---@class snowcap.widget.v1.Padding
---@field top number?
---@field right number?
---@field bottom number?
---@field left number?

---@class snowcap.widget.v1.Length
---@field fill google.protobuf.Empty?
---@field fill_portion integer?
---@field shrink google.protobuf.Empty?
---@field fixed number?

---@class snowcap.widget.v1.Color
---@field red number?
---@field green number?
---@field blue number?
---@field alpha number?

---@class snowcap.widget.v1.Font
---@field family snowcap.widget.v1.Font.Family?
---@field weight snowcap.widget.v1.Font.Weight?
---@field stretch snowcap.widget.v1.Font.Stretch?
---@field style snowcap.widget.v1.Font.Style?

---@class snowcap.widget.v1.Font.Family
---@field name string?
---@field serif google.protobuf.Empty?
---@field sans_serif google.protobuf.Empty?
---@field cursive google.protobuf.Empty?
---@field fantasy google.protobuf.Empty?
---@field monospace google.protobuf.Empty?

---@class snowcap.widget.v1.Radius
---@field top_left number?
---@field top_right number?
---@field bottom_right number?
---@field bottom_left number?

---@class snowcap.widget.v1.Border
---@field color snowcap.widget.v1.Color?
---@field width number?
---@field radius snowcap.widget.v1.Radius?

---@class snowcap.widget.v1.Theme
---@field palette snowcap.widget.v1.Palette?
---@field text_style snowcap.widget.v1.Text.Style?
---@field scrollable_style snowcap.widget.v1.Scrollable.Style?
---@field container_style snowcap.widget.v1.Container.Style?
---@field button_style snowcap.widget.v1.Button.Style?

---@class snowcap.widget.v1.Palette
---@field background snowcap.widget.v1.Color?
---@field text snowcap.widget.v1.Color?
---@field primary snowcap.widget.v1.Color?
---@field success snowcap.widget.v1.Color?
---@field warning snowcap.widget.v1.Color?
---@field danger snowcap.widget.v1.Color?

---@class snowcap.widget.v1.WidgetDef
---@field theme snowcap.widget.v1.Theme?
---@field text snowcap.widget.v1.Text?
---@field column snowcap.widget.v1.Column?
---@field row snowcap.widget.v1.Row?
---@field scrollable snowcap.widget.v1.Scrollable?
---@field container snowcap.widget.v1.Container?
---@field button snowcap.widget.v1.Button?

---@class snowcap.widget.v1.Text
---@field text string?
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field horizontal_alignment snowcap.widget.v1.Alignment?
---@field vertical_alignment snowcap.widget.v1.Alignment?
---@field style snowcap.widget.v1.Text.Style?

---@class snowcap.widget.v1.Text.Style
---@field color snowcap.widget.v1.Color?
---@field pixels number?
---@field font snowcap.widget.v1.Font?

---@class snowcap.widget.v1.Column
---@field spacing number?
---@field padding snowcap.widget.v1.Padding?
---@field item_alignment snowcap.widget.v1.Alignment?
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field max_width number?
---@field clip boolean?
---@field children snowcap.widget.v1.WidgetDef[]?

---@class snowcap.widget.v1.Row
---@field spacing number?
---@field padding snowcap.widget.v1.Padding?
---@field item_alignment snowcap.widget.v1.Alignment?
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field clip boolean?
---@field children snowcap.widget.v1.WidgetDef[]?

---@class snowcap.widget.v1.Scrollable
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field direction snowcap.widget.v1.Scrollable.Direction?
---@field child snowcap.widget.v1.WidgetDef?
---@field style snowcap.widget.v1.Scrollable.Style?

---@class snowcap.widget.v1.Scrollable.Style
---@field container_style snowcap.widget.v1.Container.Style?
---@field vertical_rail snowcap.widget.v1.Scrollable.Rail?
---@field horizontal_rail snowcap.widget.v1.Scrollable.Rail?

---@class snowcap.widget.v1.Scrollable.Rail
---@field background_color snowcap.widget.v1.Color?
---@field border snowcap.widget.v1.Border?
---@field scroller_color snowcap.widget.v1.Color?
---@field scroller_border snowcap.widget.v1.Border?

---@class snowcap.widget.v1.Scrollable.Direction
---@field vertical snowcap.widget.v1.Scrollable.Scrollbar?
---@field horizontal snowcap.widget.v1.Scrollable.Scrollbar?

---@class snowcap.widget.v1.Scrollable.Scrollbar
---@field width_pixels number?
---@field margin_pixels number?
---@field scroller_width_pixels number?
---@field anchor_to_end boolean?
---@field embed_spacing number?

---@class snowcap.widget.v1.Container
---@field padding snowcap.widget.v1.Padding?
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field max_width number?
---@field max_height number?
---@field horizontal_alignment snowcap.widget.v1.Alignment?
---@field vertical_alignment snowcap.widget.v1.Alignment?
---@field clip boolean?
---@field child snowcap.widget.v1.WidgetDef?
---@field style snowcap.widget.v1.Container.Style?

---@class snowcap.widget.v1.Container.Style
---@field text_color snowcap.widget.v1.Color?
---@field background_color snowcap.widget.v1.Color?
---@field border snowcap.widget.v1.Border?

---@class snowcap.widget.v1.Button
---@field child snowcap.widget.v1.WidgetDef?
---@field width snowcap.widget.v1.Length?
---@field height snowcap.widget.v1.Length?
---@field padding snowcap.widget.v1.Padding?
---@field clip boolean?
---@field style snowcap.widget.v1.Button.Style?
---@field widget_id integer?

---@class snowcap.widget.v1.Button.Style
---@field active snowcap.widget.v1.Button.Style.Inner?
---@field hovered snowcap.widget.v1.Button.Style.Inner?
---@field pressed snowcap.widget.v1.Button.Style.Inner?
---@field disabled snowcap.widget.v1.Button.Style.Inner?

---@class snowcap.widget.v1.Button.Style.Inner
---@field text_color snowcap.widget.v1.Color?
---@field background_color snowcap.widget.v1.Color?
---@field border snowcap.widget.v1.Border?

---@class snowcap.widget.v1.Button.Event

---@class snowcap.widget.v1.GetWidgetEventsRequest
---@field layer_id integer?

---@class snowcap.widget.v1.GetWidgetEventsResponse
---@field widget_id integer?
---@field button snowcap.widget.v1.Button.Event?

---@class snowcap.layer.v1.NewLayerRequest
---@field widget_def snowcap.widget.v1.WidgetDef?
---@field width integer?
---@field height integer?
---@field anchor snowcap.layer.v1.Anchor?
---@field keyboard_interactivity snowcap.layer.v1.KeyboardInteractivity?
---@field exclusive_zone integer?
---@field layer snowcap.layer.v1.Layer?

---@class snowcap.layer.v1.NewLayerResponse
---@field layer_id integer?

---@class snowcap.layer.v1.CloseRequest
---@field layer_id integer?

---@class snowcap.v0alpha1.Nothing

---@class snowcap.v1.Nothing

local google = {}
google.protobuf = {}
google.protobuf.Empty = {}
local snowcap = {}
snowcap.input = {}
snowcap.input.v0alpha1 = {}
snowcap.input.v0alpha1.Modifiers = {}
snowcap.input.v0alpha1.KeyboardKeyRequest = {}
snowcap.input.v0alpha1.KeyboardKeyResponse = {}
snowcap.input.v0alpha1.PointerButtonRequest = {}
snowcap.input.v0alpha1.PointerButtonResponse = {}
snowcap.input.v1 = {}
snowcap.input.v1.Modifiers = {}
snowcap.input.v1.KeyboardKeyRequest = {}
snowcap.input.v1.KeyboardKeyResponse = {}
snowcap.input.v1.PointerButtonRequest = {}
snowcap.input.v1.PointerButtonResponse = {}
snowcap.widget = {}
snowcap.widget.v0alpha1 = {}
snowcap.widget.v0alpha1.Padding = {}
snowcap.widget.v0alpha1.Length = {}
snowcap.widget.v0alpha1.Color = {}
snowcap.widget.v0alpha1.Font = {}
snowcap.widget.v0alpha1.Font.Family = {}
snowcap.widget.v0alpha1.WidgetDef = {}
snowcap.widget.v0alpha1.Text = {}
snowcap.widget.v0alpha1.Column = {}
snowcap.widget.v0alpha1.Row = {}
snowcap.widget.v0alpha1.ScrollableDirection = {}
snowcap.widget.v0alpha1.ScrollableProperties = {}
snowcap.widget.v0alpha1.Scrollable = {}
snowcap.widget.v0alpha1.Container = {}
snowcap.layer = {}
snowcap.layer.v0alpha1 = {}
snowcap.layer.v0alpha1.NewLayerRequest = {}
snowcap.layer.v0alpha1.NewLayerResponse = {}
snowcap.layer.v0alpha1.CloseRequest = {}
snowcap.widget.v1 = {}
snowcap.widget.v1.Padding = {}
snowcap.widget.v1.Length = {}
snowcap.widget.v1.Color = {}
snowcap.widget.v1.Font = {}
snowcap.widget.v1.Font.Family = {}
snowcap.widget.v1.Radius = {}
snowcap.widget.v1.Border = {}
snowcap.widget.v1.Theme = {}
snowcap.widget.v1.Palette = {}
snowcap.widget.v1.WidgetDef = {}
snowcap.widget.v1.Text = {}
snowcap.widget.v1.Text.Style = {}
snowcap.widget.v1.Column = {}
snowcap.widget.v1.Row = {}
snowcap.widget.v1.Scrollable = {}
snowcap.widget.v1.Scrollable.Style = {}
snowcap.widget.v1.Scrollable.Rail = {}
snowcap.widget.v1.Scrollable.Direction = {}
snowcap.widget.v1.Scrollable.Scrollbar = {}
snowcap.widget.v1.Container = {}
snowcap.widget.v1.Container.Style = {}
snowcap.widget.v1.Button = {}
snowcap.widget.v1.Button.Style = {}
snowcap.widget.v1.Button.Style.Inner = {}
snowcap.widget.v1.Button.Event = {}
snowcap.widget.v1.GetWidgetEventsRequest = {}
snowcap.widget.v1.GetWidgetEventsResponse = {}
snowcap.layer.v1 = {}
snowcap.layer.v1.NewLayerRequest = {}
snowcap.layer.v1.NewLayerResponse = {}
snowcap.layer.v1.CloseRequest = {}
snowcap.v0alpha1 = {}
snowcap.v0alpha1.Nothing = {}
snowcap.v1 = {}
snowcap.v1.Nothing = {}
snowcap.widget.v0alpha1.Alignment = snowcap_widget_v0alpha1_Alignment
snowcap.widget.v0alpha1.ScrollableAlignment = snowcap_widget_v0alpha1_ScrollableAlignment
snowcap.widget.v0alpha1.Font.Weight = snowcap_widget_v0alpha1_Font_Weight
snowcap.widget.v0alpha1.Font.Stretch = snowcap_widget_v0alpha1_Font_Stretch
snowcap.widget.v0alpha1.Font.Style = snowcap_widget_v0alpha1_Font_Style
snowcap.layer.v0alpha1.Anchor = snowcap_layer_v0alpha1_Anchor
snowcap.layer.v0alpha1.KeyboardInteractivity = snowcap_layer_v0alpha1_KeyboardInteractivity
snowcap.layer.v0alpha1.Layer = snowcap_layer_v0alpha1_Layer
snowcap.widget.v1.Alignment = snowcap_widget_v1_Alignment
snowcap.widget.v1.Font.Weight = snowcap_widget_v1_Font_Weight
snowcap.widget.v1.Font.Stretch = snowcap_widget_v1_Font_Stretch
snowcap.widget.v1.Font.Style = snowcap_widget_v1_Font_Style
snowcap.layer.v1.Anchor = snowcap_layer_v1_Anchor
snowcap.layer.v1.KeyboardInteractivity = snowcap_layer_v1_KeyboardInteractivity
snowcap.layer.v1.Layer = snowcap_layer_v1_Layer

snowcap.input.v0alpha1.InputService = {}
snowcap.input.v0alpha1.InputService.KeyboardKey = {}
snowcap.input.v0alpha1.InputService.KeyboardKey.service = "snowcap.input.v0alpha1.InputService"
snowcap.input.v0alpha1.InputService.KeyboardKey.method = "KeyboardKey"
snowcap.input.v0alpha1.InputService.KeyboardKey.request = ".snowcap.input.v0alpha1.KeyboardKeyRequest"
snowcap.input.v0alpha1.InputService.KeyboardKey.response = ".snowcap.input.v0alpha1.KeyboardKeyResponse"

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data snowcap.input.v0alpha1.KeyboardKeyRequest
---@param callback fun(response: snowcap.input.v0alpha1.KeyboardKeyResponse)
---
---@return string | nil An error string, if any
function Client:snowcap_input_v0alpha1_InputService_KeyboardKey(data, callback)
    return self:server_streaming_request(snowcap.input.v0alpha1.InputService.KeyboardKey, data, callback)
end
snowcap.input.v0alpha1.InputService.PointerButton = {}
snowcap.input.v0alpha1.InputService.PointerButton.service = "snowcap.input.v0alpha1.InputService"
snowcap.input.v0alpha1.InputService.PointerButton.method = "PointerButton"
snowcap.input.v0alpha1.InputService.PointerButton.request = ".snowcap.input.v0alpha1.PointerButtonRequest"
snowcap.input.v0alpha1.InputService.PointerButton.response = ".snowcap.input.v0alpha1.PointerButtonResponse"

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data snowcap.input.v0alpha1.PointerButtonRequest
---@param callback fun(response: snowcap.input.v0alpha1.PointerButtonResponse)
---
---@return string | nil An error string, if any
function Client:snowcap_input_v0alpha1_InputService_PointerButton(data, callback)
    return self:server_streaming_request(snowcap.input.v0alpha1.InputService.PointerButton, data, callback)
end
snowcap.input.v1.InputService = {}
snowcap.input.v1.InputService.KeyboardKey = {}
snowcap.input.v1.InputService.KeyboardKey.service = "snowcap.input.v1.InputService"
snowcap.input.v1.InputService.KeyboardKey.method = "KeyboardKey"
snowcap.input.v1.InputService.KeyboardKey.request = ".snowcap.input.v1.KeyboardKeyRequest"
snowcap.input.v1.InputService.KeyboardKey.response = ".snowcap.input.v1.KeyboardKeyResponse"

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data snowcap.input.v1.KeyboardKeyRequest
---@param callback fun(response: snowcap.input.v1.KeyboardKeyResponse)
---
---@return string | nil An error string, if any
function Client:snowcap_input_v1_InputService_KeyboardKey(data, callback)
    return self:server_streaming_request(snowcap.input.v1.InputService.KeyboardKey, data, callback)
end
snowcap.input.v1.InputService.PointerButton = {}
snowcap.input.v1.InputService.PointerButton.service = "snowcap.input.v1.InputService"
snowcap.input.v1.InputService.PointerButton.method = "PointerButton"
snowcap.input.v1.InputService.PointerButton.request = ".snowcap.input.v1.PointerButtonRequest"
snowcap.input.v1.InputService.PointerButton.response = ".snowcap.input.v1.PointerButtonResponse"

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data snowcap.input.v1.PointerButtonRequest
---@param callback fun(response: snowcap.input.v1.PointerButtonResponse)
---
---@return string | nil An error string, if any
function Client:snowcap_input_v1_InputService_PointerButton(data, callback)
    return self:server_streaming_request(snowcap.input.v1.InputService.PointerButton, data, callback)
end
snowcap.layer.v0alpha1.LayerService = {}
snowcap.layer.v0alpha1.LayerService.NewLayer = {}
snowcap.layer.v0alpha1.LayerService.NewLayer.service = "snowcap.layer.v0alpha1.LayerService"
snowcap.layer.v0alpha1.LayerService.NewLayer.method = "NewLayer"
snowcap.layer.v0alpha1.LayerService.NewLayer.request = ".snowcap.layer.v0alpha1.NewLayerRequest"
snowcap.layer.v0alpha1.LayerService.NewLayer.response = ".snowcap.layer.v0alpha1.NewLayerResponse"

---Performs a unary request.
---
---@nodiscard
---
---@param data snowcap.layer.v0alpha1.NewLayerRequest
---
---@return snowcap.layer.v0alpha1.NewLayerResponse | nil response
---@return string | nil error An error string, if any
function Client:snowcap_layer_v0alpha1_LayerService_NewLayer(data)
    return self:unary_request(snowcap.layer.v0alpha1.LayerService.NewLayer, data)
end
snowcap.layer.v0alpha1.LayerService.Close = {}
snowcap.layer.v0alpha1.LayerService.Close.service = "snowcap.layer.v0alpha1.LayerService"
snowcap.layer.v0alpha1.LayerService.Close.method = "Close"
snowcap.layer.v0alpha1.LayerService.Close.request = ".snowcap.layer.v0alpha1.CloseRequest"
snowcap.layer.v0alpha1.LayerService.Close.response = ".google.protobuf.Empty"

---Performs a unary request.
---
---@nodiscard
---
---@param data snowcap.layer.v0alpha1.CloseRequest
---
---@return google.protobuf.Empty | nil response
---@return string | nil error An error string, if any
function Client:snowcap_layer_v0alpha1_LayerService_Close(data)
    return self:unary_request(snowcap.layer.v0alpha1.LayerService.Close, data)
end
snowcap.widget.v1.WidgetService = {}
snowcap.widget.v1.WidgetService.GetWidgetEvents = {}
snowcap.widget.v1.WidgetService.GetWidgetEvents.service = "snowcap.widget.v1.WidgetService"
snowcap.widget.v1.WidgetService.GetWidgetEvents.method = "GetWidgetEvents"
snowcap.widget.v1.WidgetService.GetWidgetEvents.request = ".snowcap.widget.v1.GetWidgetEventsRequest"
snowcap.widget.v1.WidgetService.GetWidgetEvents.response = ".snowcap.widget.v1.GetWidgetEventsResponse"

---Performs a server-streaming request.
---
---`callback` will be called with every streamed response.
---
---@nodiscard
---
---@param data snowcap.widget.v1.GetWidgetEventsRequest
---@param callback fun(response: snowcap.widget.v1.GetWidgetEventsResponse)
---
---@return string | nil An error string, if any
function Client:snowcap_widget_v1_WidgetService_GetWidgetEvents(data, callback)
    return self:server_streaming_request(snowcap.widget.v1.WidgetService.GetWidgetEvents, data, callback)
end
snowcap.layer.v1.LayerService = {}
snowcap.layer.v1.LayerService.NewLayer = {}
snowcap.layer.v1.LayerService.NewLayer.service = "snowcap.layer.v1.LayerService"
snowcap.layer.v1.LayerService.NewLayer.method = "NewLayer"
snowcap.layer.v1.LayerService.NewLayer.request = ".snowcap.layer.v1.NewLayerRequest"
snowcap.layer.v1.LayerService.NewLayer.response = ".snowcap.layer.v1.NewLayerResponse"

---Performs a unary request.
---
---@nodiscard
---
---@param data snowcap.layer.v1.NewLayerRequest
---
---@return snowcap.layer.v1.NewLayerResponse | nil response
---@return string | nil error An error string, if any
function Client:snowcap_layer_v1_LayerService_NewLayer(data)
    return self:unary_request(snowcap.layer.v1.LayerService.NewLayer, data)
end
snowcap.layer.v1.LayerService.Close = {}
snowcap.layer.v1.LayerService.Close.service = "snowcap.layer.v1.LayerService"
snowcap.layer.v1.LayerService.Close.method = "Close"
snowcap.layer.v1.LayerService.Close.request = ".snowcap.layer.v1.CloseRequest"
snowcap.layer.v1.LayerService.Close.response = ".google.protobuf.Empty"

---Performs a unary request.
---
---@nodiscard
---
---@param data snowcap.layer.v1.CloseRequest
---
---@return google.protobuf.Empty | nil response
---@return string | nil error An error string, if any
function Client:snowcap_layer_v1_LayerService_Close(data)
    return self:unary_request(snowcap.layer.v1.LayerService.Close, data)
end
return {
    google = google,
    snowcap = snowcap,
    grpc_client = grpc_client,
}

