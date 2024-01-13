---The protobuf absolute path prefix
local prefix = "pinnacle.prefix." .. require("pinnacle").version .. "."
local service = prefix .. "WindowService"

---@type table<string, { request_type: string?, response_type: string? }>
---@enum (key) WindowServiceMethod
local rpc_types = {
    Close = {},
    SetGeometry = {},
    SetFullscreen = {},
    SetMaximized = {},
    SetFloating = {},
    MoveToTag = {},
    SetTag = {},
    MoveGrab = {},
    ResizeGrab = {},
    Get = {
        response_type = "GetResponse",
    },
    GetProperties = {
        response_type = "GetPropertiesResponse",
    },
    AddWindowRule = {},
}

---Build GrpcRequestParams
---@param method WindowServiceMethod
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

---@class WindowHandleModule
local window_handle = {}

---@class WindowHandle
---@field private config_client Client
---@field id integer
local WindowHandle = {}

---@class WindowModule
---@field private handle WindowHandleModule
local window = {}
window.handle = window_handle

---@class Window
---@field private config_client Client
local Window = {}

---Get all windows.
---
---@return WindowHandle[]
function Window:get_all()
    local response = self.config_client:unary_request(build_grpc_request_params("Get", {}))

    local handles = window_handle.new_from_table(self.config_client, response.window_ids)

    return handles
end

--- TODO: docs
---@param button MouseButton
function Window:begin_move(button)
    self.config_client:unary_request(build_grpc_request_params("MoveGrab", { button = button }))
end

--- TODO: docs
---@param button MouseButton
function Window:begin_resize(button)
    self.config_client:unary_request(build_grpc_request_params("ResizeGrab", { button = button }))
end

---@class WindowRuleCondition
---@field any WindowRuleCondition[]?
---@field all WindowRuleCondition[]?
---@field classes string[]?
---@field titles string[]?
---@field tags TagHandle[]?

---@class WindowRule
---@field output OutputHandle?
---@field tags TagHandle[]?
---@field floating boolean?
---@field fullscreen_or_maximized FullscreenOrMaximized?
---@field x integer?
---@field y integer?
---@field width integer?
---@field height integer?

---@enum (key) FullscreenOrMaximized
local _fullscreen_or_maximized = {
    neither = 1,
    fullscreen = 2,
    maximized = 3,
}

local _fullscreen_or_maximized_keys = {
    [1] = "neither",
    [2] = "fullscreen",
    [3] = "maximized",
}

---@param rule { cond: WindowRuleCondition, rule: WindowRule }
function Window:add_window_rule(rule)
    if rule.cond.tags then
        local ids = {}
        for _, tg in pairs(rule.cond.tags) do
            table.insert(ids, tg.id)
        end
        rule.cond.tags = ids
    end

    if rule.rule.output then
        rule.rule.output = rule.rule.output.name
    end

    if rule.rule.tags then
        local ids = {}
        for _, tg in pairs(rule.cond.tags) do
            table.insert(ids, tg.id)
        end
        rule.cond.tags = ids
    end

    if rule.rule.fullscreen_or_maximized then
        rule.rule.fullscreen_or_maximized = _fullscreen_or_maximized[rule.rule.fullscreen_or_maximized]
    end

    self.config_client:unary_request(build_grpc_request_params("AddWindowRule", {
        cond = rule.cond,
        rule = rule.rule,
    }))
end

---Send a close request to this window.
function WindowHandle:close()
    self.config_client:unary_request(build_grpc_request_params("Close", { window_id = self.id }))
end

---Set this window's location and/or size.
---
---@param geo { x: integer?, y: integer, width: integer?, height: integer? }
function WindowHandle:set_geometry(geo)
    self.config_client:unary_request(build_grpc_request_params("SetGeometry", { window_id = self.id, geometry = geo }))
end

---Set this window to fullscreen or not.
---@param fullscreen boolean
function WindowHandle:set_fullscreen(fullscreen)
    self.config_client:unary_request(
        build_grpc_request_params("SetFullscreen", { window_id = self.id, set = fullscreen })
    )
end

function WindowHandle:toggle_fullscreen()
    self.config_client:unary_request(build_grpc_request_params("SetFullscreen", { window_id = self.id, toggle = {} }))
end

function WindowHandle:set_maximized(maximized)
    self.config_client:unary_request(
        build_grpc_request_params("SetMaximized", { window_id = self.id, set = maximized })
    )
end

function WindowHandle:toggle_maximized()
    self.config_client:unary_request(build_grpc_request_params("SetMaximized", { window_id = self.id, toggle = {} }))
end

function WindowHandle:set_floating(floating)
    self.config_client:unary_request(build_grpc_request_params("SetFloating", { window_id = self.id, set = floating }))
end

function WindowHandle:toggle_floating()
    self.config_client:unary_request(build_grpc_request_params("SetFloating", { window_id = self.id, toggle = {} }))
end

---@param tag TagHandle
function WindowHandle:move_to_tag(tag)
    self.config_client:unary_request(build_grpc_request_params("MoveToTag", { window_id = self.id, tag_id = tag.id }))
end

---Tag or untag the given tag on this window.
---@param tag TagHandle
---@param set boolean
function WindowHandle:set_tag(tag, set)
    self.config_client:unary_request(
        build_grpc_request_params("SetTag", { window_id = self.id, tag_id = tag.id, set = set })
    )
end

---Toggle the given tag on this window.
---@param tag TagHandle
function WindowHandle:toggle_tag(tag)
    self.config_client:unary_request(
        build_grpc_request_params("SetTag", { window_id = self.id, tag_id = tag.id, toggle = {} })
    )
end

---@class WindowProperties
---@field geometry { x: integer?, y: integer?, width: integer?, height: integer? }?
---@field class string?
---@field title string?
---@field focused boolean?
---@field floating boolean?
---@field fullscreen_or_maximized FullscreenOrMaximized?

---@return WindowProperties
function WindowHandle:props()
    local response =
        self.config_client:unary_request(build_grpc_request_params("GetProperties", { window_id = self.id }))

    response.fullscreen_or_maximized = _fullscreen_or_maximized_keys[response.fullscreen_or_maximized]

    return response
end

---@param config_client Client
---@return Window
function window.new(config_client)
    ---@type Window
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Window })
    return self
end

---Create a new `WindowHandle` from an id.
---@param config_client Client
---@param window_id integer
---@return WindowHandle
function window_handle.new(config_client, window_id)
    ---@type WindowHandle
    local self = {
        config_client = config_client,
        id = window_id,
    }
    setmetatable(self, { __index = WindowHandle })
    return self
end

---@param config_client Client
---@param window_ids integer[]
---
---@return WindowHandle[]
function window_handle.new_from_table(config_client, window_ids)
    ---@type WindowHandle[]
    local handles = {}

    for _, id in pairs(window_ids) do
        table.insert(handles, window_handle.new(config_client, id))
    end

    return handles
end

return window
