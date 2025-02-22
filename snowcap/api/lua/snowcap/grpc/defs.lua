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
			local err_name = require("grpc_client.status").name(grpc_status)
			local grpc_msg = headers:get("grpc-message")
			local grpc_msg = grpc_msg and (", msg = " .. grpc_msg) or ""
			local err_str = "error from response: code = " .. (err_name or "unknown grpc status code") .. grpc_msg
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
			local err_name = require("grpc_client.status").name(grpc_status)
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
			local err_name = require("grpc_client.status").name(grpc_status)
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

---@alias google.protobuf.Empty nil

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

---@class snowcap.v0alpha1.Nothing

local snowcap = {}
snowcap.input = {}
snowcap.input.v0alpha1 = {}
snowcap.input.v0alpha1.Modifiers = {}
snowcap.input.v0alpha1.KeyboardKeyRequest = {}
snowcap.input.v0alpha1.KeyboardKeyResponse = {}
snowcap.input.v0alpha1.PointerButtonRequest = {}
snowcap.input.v0alpha1.PointerButtonResponse = {}
local google = {}
google.protobuf = {}
google.protobuf.Empty = {}
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
snowcap.v0alpha1 = {}
snowcap.v0alpha1.Nothing = {}
snowcap.widget.v0alpha1.Alignment = snowcap_widget_v0alpha1_Alignment
snowcap.widget.v0alpha1.ScrollableAlignment = snowcap_widget_v0alpha1_ScrollableAlignment
snowcap.widget.v0alpha1.Font.Weight = snowcap_widget_v0alpha1_Font_Weight
snowcap.widget.v0alpha1.Font.Stretch = snowcap_widget_v0alpha1_Font_Stretch
snowcap.widget.v0alpha1.Font.Style = snowcap_widget_v0alpha1_Font_Style
snowcap.layer.v0alpha1.Anchor = snowcap_layer_v0alpha1_Anchor
snowcap.layer.v0alpha1.KeyboardInteractivity = snowcap_layer_v0alpha1_KeyboardInteractivity
snowcap.layer.v0alpha1.Layer = snowcap_layer_v0alpha1_Layer

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
return {
    snowcap = snowcap,
    google = google,
    grpc_client = grpc_client,
}

