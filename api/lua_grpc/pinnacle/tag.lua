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
        request_type = req_type and prefix .. req_type or prefix .. method .. "Request",
        response_type = resp_type and prefix .. resp_type,
        data = data,
    }
end

---@class TagHandleModule
local tag_handle = {}

---@classmod
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

    for _, id in ipairs(response.tag_ids or {}) do
        table.insert(handles, tag_handle.new(self.config_client, id))
    end

    return handles
end

---Get the tag with the given name and output.
---
---If `output` is not specified, this uses the focused output.
---
---If an output has more than one tag with the same name, this returns the first.
---
---### Example
---```lua
--- -- Get tags on the focused output
---local tag = Tag:get("Tag")
---
--- -- Get tags on a specific output
---local tag_on_hdmi1 = Tag:get("Tag", Output:get_by_name("HDMI-1"))
---```
---
---@param name string
---@param output OutputHandle?
---
---@return TagHandle | nil
function Tag:get(name, output)
    output = output or require("pinnacle.output").new(self.config_client):get_focused()

    if not output then
        return
    end

    local handles = self:get_all()

    for _, handle in ipairs(handles) do
        local props = handle:props()
        if props.output and props.output.name == output.name and props.name == name then
            return handle
        end
    end

    return nil
end

---Add tags with the given names to the specified output.
---
---Returns handles to the created tags.
---
---### Example
---```lua
---local tags = Tag:add(Output:get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
--- -- With a table
---local tag_names = { "1", "2", "Buckle", "Shoe" }
---local tags = Tag:add(Output:get_by_name("HDMI-1"), tag_names)
---```
---
---@param output OutputHandle
---@param ... string
---
---@return TagHandle[] tags Handles to the created tags
---
---@overload fun(self: self, output: OutputHandle, tag_names: string[])
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

    for _, id in ipairs(response.tag_ids) do
        table.insert(handles, tag_handle.new(self.config_client, id))
    end

    return handles
end

---Remove the given tags.
---
---### Example
---```lua
---local tags = Tag:add(Output:get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
---Tag:remove(tags) -- "HDMI-1" no longer has those tags
---```
---
---@param tags TagHandle[]
function Tag:remove(tags)
    ---@type integer[]
    local ids = {}

    for _, tg in ipairs(tags) do
        table.insert(ids, tg.id)
    end

    self.config_client:unary_request(build_grpc_request_params("Remove", { tag_ids = ids }))
end

---@class LayoutCycler
---@field next fun(output: OutputHandle?)
---@field prev fun(output: OutputHandle?)

---Create a layout cycler that will cycle layouts on the given output.
---
---This returns a `LayoutCycler` table with two fields, both functions that take in an optional `OutputHandle`:
--- - `next`: Cycle to the next layout on the given output
--- - `prev`: Cycle to the previous layout on the given output
---
---If the output isn't specified then the focused one will be used.
---
---Internally, this will only change the layout of the first active tag on the output
---because that is the one that determines the layout.
---
---### Example
---```lua
--- ---@type LayoutCycler[]
---local layouts = {
---    "master_stack",
---    "dwindle",
---    "corner_top_left",
---    "corner_top_right".
---} -- Only cycle between these four layouts
---
---local layout_cycler = Tag:new_layout_cycler()
---
--- -- Assume the focused output starts with the "master_stack" layout
---layout_cycler.next() -- Layout is now "dwindle"
---layout_cycler.next() -- Layout is now "corner_top_left"
---layout_cycler.next() -- Layout is now "corner_top_right"
---layout_cycler.next() -- Layout is now "dwindle"
---layout_cycler.next() -- Layout is now "corner_top_right"
---
--- -- Cycling on another output
---layout_cycler.next(Output:get_by_name("eDP-1"))
---layout_cycler.prev(Output:get_by_name("HDMI-1"))
---```
---
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
            local output = output or require("pinnacle.output").new(self.config_client):get_focused()
            if not output then
                return
            end

            local tags = output:props().tags

            for _, tg in ipairs(tags) do
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
            local output = output or require("pinnacle.output").new(self.config_client):get_focused()
            if not output then
                return
            end

            local tags = output:props().tags

            for _, tg in ipairs(tags) do
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
---
---### Example
---```lua
---local tags = Tag:add(Output:get_by_name("HDMI-1"), "1", "2", "Buckle", "Shoe")
---
---tags[2]:remove()
---tags[4]:remove()
--- -- "HDMI-1" now only has tags "1" and "Buckle"
---```
function TagHandle:remove()
    self.config_client:unary_request(build_grpc_request_params("Remove", { tag_ids = { self.id } }))
end

local _layouts = {
    master_stack = 1,
    dwindle = 2,
    spiral = 3,
    corner_top_left = 4,
    corner_top_right = 5,
    corner_bottom_left = 6,
    corner_bottom_right = 7,
}
---@alias Layout
---| "master_stack" # One master window on the left with all other windows stacked to the right.
---| "dwindle" # Windows split in half towards the bottom right corner.
---| "spiral" # Windows split in half in a spiral.
---| "corner_top_left" # One main corner window in the top left with a column of windows on the right and a row on the bottom.
---| "corner_top_right" # One main corner window in the top right with a column of windows on the left and a row on the bottom.
---| "corner_bottom_left" # One main corner window in the bottom left with a column of windows on the right and a row on the top.
---| "corner_bottom_right" # One main corner window in the bottom right with a column of windows on the left and a row on the top.

---Set this tag's layout.
---
---If this is the first active tag on its output, its layout will be used to tile windows.
---
---### Example
---```lua
--- -- Assume the focused output has tag "Tag"
---Tag:get("Tag"):set_layout("dwindle")
---```
---
---@param layout Layout
function TagHandle:set_layout(layout)
    local layout = _layouts[layout]

    self.config_client:unary_request(build_grpc_request_params("SetLayout", {
        tag_id = self.id,
        layout = layout,
    }))
end

---Activate this tag and deactivate all other ones on the same output.
---
---### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag:get("2"):switch_to() -- Displays Firefox and Discord
---Tag:get("3"):switch_to() -- Displays Steam
---```
function TagHandle:switch_to()
    self.config_client:unary_request(build_grpc_request_params("SwitchTo", { tag_id = self.id }))
end

---Set whether or not this tag is active.
---
---### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag:get("2"):set_active(true)  -- Displays Firefox and Discord
---Tag:get("3"):set_active(true)  -- Displays Firefox, Discord, and Steam
---Tag:get("2"):set_active(false) -- Displays Steam
---```
---
---@param active boolean
function TagHandle:set_active(active)
    self.config_client:unary_request(build_grpc_request_params("SetActive", { tag_id = self.id, set = active }))
end

---Toggle this tag's active state.
---
---### Example
---```lua
--- -- Assume the focused output has the following inactive tags and windows:
--- --  - "1": Alacritty
--- --  - "2": Firefox, Discord
--- --  - "3": Steam
---Tag:get("2"):toggle_active() -- Displays Firefox and Discord
---Tag:get("2"):toggle_active() -- Displays nothing
---```
function TagHandle:toggle_active()
    self.config_client:unary_request(build_grpc_request_params("SetActive", { tag_id = self.id, toggle = {} }))
end

---@class TagProperties
---@field active boolean? Whether or not the tag is currently being displayed
---@field name string? The name of the tag
---@field output OutputHandle? The output the tag is on

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

---Get whether or not this tag is being displayed.
---
---Shorthand for `handle:props().active`.
---
---@return boolean?
function TagHandle:active()
    return self:props().active
end

---Get this tag's name.
---
---Shorthand for `handle:props().name`.
---
---@return string?
function TagHandle:name()
    return self:props().name
end

---Get the output this tag is on.
---
---Shorthand for `handle:props().output`.
---
---@return OutputHandle?
function TagHandle:output()
    return self:props().output
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

    for _, id in ipairs(tag_ids) do
        table.insert(handles, tag_handle.new(config_client, id))
    end

    return handles
end

return tag
