---@class InputModule
local input = {}

---@class Input
---@field private config_client Client
local Input = {}

---@enum Modifier
local modifier = {
    SHIFT = 1,
    CTRL = 2,
    ALT = 3,
    SUPER = 4,
}

---@param mods Modifier[]
---@param key integer | string
---@param action fun()
function Input:set_keybind(mods, key, action)
    local raw_code = nil
    local xkb_name = nil

    if type(key) == "number" then
        raw_code = key
    elseif type(key) == "string" then
        xkb_name = key
    end

    self.config_client:server_streaming_request({
        service = "pinnacle.input.v0alpha1.InputService",
        method = "SetKeybind",
        request_type = "pinnacle.input.v0alpha1.SetKeybindRequest",
        data = {
            modifiers = mods,
            -- oneof not violated because `key` can't be both an int and string
            raw_code = raw_code,
            xkb_name = xkb_name,
        },
    }, action)
end

function input.new(config_client)
    ---@type Input
    local self = {
        config_client = config_client,
    }
    setmetatable(self, { __index = Input })
    return self
end

return input
