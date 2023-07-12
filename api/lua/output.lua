-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@class Output A display.
---@field name string The name of this output (or rather, of its connector).
local op = {}

---Add methods to this output.
---@param props Output
---@return Output
local function new_output(props)
    -- Copy functions over
    for k, v in pairs(op) do
        props[k] = v
    end

    return props
end

------------------------------------------------------

local output = {}

---Get an output by its name.
---
---"Name" in this sense does not mean its model or manufacturer;
---rather, "name" is the name of the connector the output is connected to.
---This should be something like "HDMI-A-0", "eDP-1", or similar.
---
---# Examples
---```lua
---local monitor = output.get_by_name("DP-1")
---print(monitor.name) -- should print `DP-1`
---```
---@param name string The name of the output.
---@return Output|nil
function output.get_by_name(name)
    SendMsg({
        Request = {
            GetOutputByName = {
                name = name,
            },
        },
    })

    local response = ReadMsg()

    local names = response.RequestResponse.response.Outputs.names

    if names[1] ~= nil then
        return new_output({ name = names[1] })
    else
        return nil
    end
end

---NOTE: This may or may not be what is reported by other monitor listing utilities. Pinnacle currently fails to pick up one of my monitors' models when it is correctly picked up by tools like wlr-randr. I'll fix this in the future.
---
---Get outputs by their model.
---This is something like "DELL E2416H" or whatever gibberish monitor manufacturers call their displays.
---@param model string The model of the output(s).
---@return Output[] outputs All outputs with this model. If there are none, the returned table will be empty.
function output.get_by_model(model)
    SendMsg({
        Request = {
            GetOutputsByModel = {
                model = model,
            },
        },
    })

    local response = ReadMsg()

    local names = response.RequestResponse.response.Outputs.names

    ---@type Output
    local outputs = {}
    for _, v in pairs(names) do
        table.insert(outputs, new_output({ name = v }))
    end

    return outputs
end

---Get outputs by their resolution.
---
---@param width integer The width of the outputs, in pixels.
---@param height integer The height of the outputs, in pixels.
---@return Output[] outputs All outputs with this resolution. If there are none, the returned table will be empty.
function output.get_by_res(width, height)
    SendMsg({
        Request = {
            GetOutputsByRes = {
                res = { width, height },
            },
        },
    })

    local response = ReadMsg()

    local names = response.RequestResponse.response.Outputs.names

    ---@type Output
    local outputs = {}
    for _, v in pairs(names) do
        table.insert(outputs, new_output({ name = v }))
    end

    return outputs
end

---Get the currently focused output. This is currently implemented as the one with the cursor on it.
---@return Output|nil output The output, or nil if none are focused.
function output.get_focused()
    SendMsg({
        Request = "GetOutputByFocus",
    })

    local response = ReadMsg()

    local names = response.RequestResponse.response.Outputs.names

    if names[1] ~= nil then
        return new_output({ name = names[1] })
    else
        return nil
    end
end

---Connect a function to be run on all current and future outputs.
---
---When called, `connect_for_all` will immediately run `func` with all currently connected outputs.
---If a new output is connected, `func` will also be called with it.
---
---Please note: this function will be run *after* Pinnacle processes your entire config.
---For example, if you define tags in `func` but toggle them directly after `connect_for_all`, nothing will happen as the tags haven't been added yet.
---@param func fun(output: Output) The function that will be run.
function output.connect_for_all(func)
    ---@param args Args
    table.insert(CallbackTable, function(args)
        local args = args.ConnectForAllOutputs
        func(new_output({ name = args.output_name }))
    end)
    SendMsg({
        ConnectForAllOutputs = {
            callback_id = #CallbackTable,
        },
    })
end

return output
