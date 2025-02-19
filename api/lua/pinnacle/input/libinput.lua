-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local log = require("pinnacle.log")
local client = require("pinnacle.grpc.client").client
local defs = require("pinnacle.grpc.defs")
local input_v1 = defs.pinnacle.input.v1
local input_service = defs.pinnacle.input.v1.InputService

---@class pinnacle.input.libinput.Libinput
local libinput = {}

---@enum (key) pinnacle.input.libinput.AccelProfile
local accel_profile_values = {
    ---No pointer acceleration
    flat = input_v1.AccelProfile.ACCEL_PROFILE_FLAT,
    ---Pointer acceleration
    adaptive = input_v1.AccelProfile.ACCEL_PROFILE_ADAPTIVE,
}

---@enum (key) pinnacle.input.libinput.ClickMethod
local click_method_values = {
    ---Button presses are generated according to where on the device the click occurs
    button_areas = input_v1.ClickMethod.CLICK_METHOD_BUTTON_AREAS,
    ---Button presses are generated according to the number of fingers used
    click_finger = input_v1.ClickMethod.CLICK_METHOD_CLICK_FINGER,
}

---@enum (key) pinnacle.input.libinput.ScrollMethod
local scroll_method_values = {
    ---Never send scroll events instead of pointer motion events
    no_scroll = input_v1.ScrollMethod.SCROLL_METHOD_NO_SCROLL,
    ---Send scroll events when two fingers are logically down on the device
    two_finger = input_v1.ScrollMethod.SCROLL_METHOD_TWO_FINGER,
    ---Send scroll events when a finger moves along the bottom or right edge of a device
    edge = input_v1.ScrollMethod.SCROLL_METHOD_EDGE,
    ---Send scroll events when a button is down and the device moves along a scroll-capable axis
    on_button_down = input_v1.ScrollMethod.SCROLL_METHOD_ON_BUTTON_DOWN,
}

---@enum (key) pinnacle.input.libinput.TapButtonMap
local tap_button_map_values = {
    ---1/2/3 finger tap maps to left/right/middle
    left_right_middle = input_v1.TapButtonMap.TAP_BUTTON_MAP_LEFT_RIGHT_MIDDLE,
    ---1/2/3 finger tap maps to left/middle/right
    left_middle_right = input_v1.TapButtonMap.TAP_BUTTON_MAP_LEFT_MIDDLE_RIGHT,
}

---@enum (key) pinnacle.input.libinput.SendEventsMode
local send_events_mode_values = {
    enabled = input_v1.SendEventsMode.SEND_EVENTS_MODE_ENABLED,
    disabled = input_v1.SendEventsMode.SEND_EVENTS_MODE_DISABLED,
    disabled_on_external_mouse = input_v1.SendEventsMode.SEND_EVENTS_MODE_DISABLED_ON_EXTERNAL_MOUSE,
}

---@class pinnacle.input.libinput.LibinputSetting
---@field accel_profile pinnacle.input.libinput.AccelProfile? Set pointer acceleration
---@field accel_speed number? Set pointer acceleration speed
---@field calibration_matrix integer[]?
---@field click_method pinnacle.input.libinput.ClickMethod?
---@field disable_while_typing boolean? Set whether or not to disable the pointing device while typing
---@field left_handed boolean? Set device left-handedness
---@field middle_emulation boolean?
---@field rotation_angle integer?
---@field scroll_button integer? Set the scroll button
---@field scroll_button_lock boolean? Set whether or not the scroll button is a hold or toggle
---@field scroll_method pinnacle.input.libinput.ScrollMethod?
---@field natural_scroll boolean? Set whether or not natural scroll is enabled, which reverses scroll direction
---@field tap_button_map pinnacle.input.libinput.TapButtonMap?
---@field tap_drag boolean?
---@field tap_drag_lock boolean?
---@field tap boolean?
---@field send_events_mode pinnacle.input.libinput.SendEventsMode?

---@class pinnacle.input.libinput.DeviceHandle
---@field sysname string
local DeviceHandle = {}

---@class pinnacle.input.libinput.Capabilities
---@field keyboard boolean
---@field pointer boolean
---@field touch boolean
---@field tablet_tool boolean
---@field tablet_pad boolean
---@field gesture boolean
---@field switch boolean

---@alias pinnacle.input.libinput.DeviceType
---| "unknown"
---| "touchpad"
---| "trackball"
---| "trackpoint"
---| "mouse"
---| "tablet"
---| "keyboard"
---| "switch"

---Gets this device's libinput capabilities.
---
---@return pinnacle.input.libinput.Capabilities
function DeviceHandle:capabilities()
    local response, err = client:unary_request(input_service.GetDeviceCapabilities, {
        device_sysname = self.sysname,
    })

    ---@type pinnacle.input.libinput.Capabilities
    local caps = {
        keyboard = false,
        pointer = false,
        touch = false,
        tablet_tool = false,
        tablet_pad = false,
        gesture = false,
        switch = false,
    }

    if err then
        log:warn(err)
        return caps
    end

    ---@cast response pinnacle.input.v1.GetDeviceCapabilitiesResponse

    caps.keyboard = response.keyboard or false
    caps.pointer = response.pointer or false
    caps.touch = response.touch or false
    caps.tablet_tool = response.tablet_tool or false
    caps.tablet_pad = response.tablet_pad or false
    caps.gesture = response.gesture or false
    caps.switch = response.switch or false

    return caps
end

---Gets the name of this device.
---
---@return string
function DeviceHandle:name()
    local response, err = client:unary_request(input_service.GetDeviceInfo, {
        device_sysname = self.sysname,
    })

    if err then
        log:warn(err)
        return ""
    end

    ---@cast response pinnacle.input.v1.GetDeviceInfoResponse

    return response.name or ""
end

---Gets this device;s product id.
---
---@return integer
function DeviceHandle:product_id()
    local response, err = client:unary_request(input_service.GetDeviceInfo, {
        device_sysname = self.sysname,
    })

    if err then
        log:warn(err)
        return 0
    end

    ---@cast response pinnacle.input.v1.GetDeviceInfoResponse

    return response.product_id or 0
end

---Gets this devices vendor id.
---@return integer
function DeviceHandle:vendor_id()
    local response, err = client:unary_request(input_service.GetDeviceInfo, {
        device_sysname = self.sysname,
    })

    if err then
        log:warn(err)
        return 0
    end

    ---@cast response pinnacle.input.v1.GetDeviceInfoResponse

    return response.vendor_id or 0
end

---Gets the type of this device.
---
---Note: This uses heuristics to determine the type and may not be correct.
---For example a device with both pointer and keyboard capabilities will be a "mouse"
---when it may actually be a keyboard.
---
---@return pinnacle.input.libinput.DeviceType
function DeviceHandle:device_type()
    local response, err = client:unary_request(input_service.GetDeviceType, {
        device_sysname = self.sysname,
    })

    if err then
        log:warn(err)
        return "unknown"
    end

    ---@cast response pinnacle.input.v1.GetDeviceTypeResponse

    ---@type pinnacle.input.libinput.DeviceType
    local type = "unknown"

    local dev_type = defs.pinnacle.input.v1.DeviceType
    if response.device_type == dev_type.DEVICE_TYPE_TOUCHPAD then
        type = "touchpad"
    elseif response.device_type == dev_type.DEVICE_TYPE_TRACKBALL then
        type = "trackball"
    elseif response.device_type == dev_type.DEVICE_TYPE_TRACKPOINT then
        type = "trackpoint"
    elseif response.device_type == dev_type.DEVICE_TYPE_MOUSE then
        type = "mouse"
    elseif response.device_type == dev_type.DEVICE_TYPE_TABLET then
        type = "tablet"
    elseif response.device_type == dev_type.DEVICE_TYPE_KEYBOARD then
        type = "keyboard"
    elseif response.device_type == dev_type.DEVICE_TYPE_SWITCH then
        type = "switch"
    end

    return type
end

---Maps the absolute input from this device to the corresponding output.
---
---This will cause touch input from this device to map proportionally
---to the area of an output. For example, tapping in the middle of the device
---will generate a tap event at the middle of the output.
---
---This only affects devices with touch capability.
---
---@param output pinnacle.output.OutputHandle The output to map the device's input to
---
---@see pinnacle.input.libinput.DeviceHandle.map_to_region To map device input to an arbitrary region instead
function DeviceHandle:map_to_output(output)
    local _, err = client:unary_request(input_service.SetDeviceMapTarget, {
        device_sysname = self.sysname,
        output_name = output.name,
    })
end

---Maps the absolute input from this device to the corresponding region
---in the global space.
---
---This will cause touch input from this device to map proportionally
---to the given region within the global space. For example, tapping in the middle of the device
---will generate a tap event at the middle of the region. This can be used
---to map a touch device to more than one output, for example.
---
---This only affects devices with touch capability.
---
---@param region { x: integer, y: integer, width: integer, height: integer } The region in the global space to map input to
---
---@see pinnacle.input.libinput.DeviceHandle.map_to_output To map device input to a specific output instead
function DeviceHandle:map_to_region(region)
    local _, err = client:unary_request(input_service.SetDeviceMapTarget, {
        device_sysname = self.sysname,
        region = {
            loc = {
                x = region.x,
                y = region.y,
            },
            size = {
                width = region.width,
                height = region.height,
            },
        },
    })
end

---@param accel_profile pinnacle.input.libinput.AccelProfile
function DeviceHandle:set_accel_profile(accel_profile)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        accel_profile = accel_profile_values[accel_profile],
    })
end

---@param accel_speed number
function DeviceHandle:set_accel_speed(accel_speed)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        accel_speed = accel_speed,
    })
end

---@param calibration_matrix number[] 6 floats
function DeviceHandle:set_calibration_matrix(calibration_matrix)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        calibration_matrix = {
            matrix = calibration_matrix,
        },
    })
end

---@param click_method pinnacle.input.libinput.ClickMethod
function DeviceHandle:set_click_method(click_method)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        click_method = click_method_values[click_method],
    })
end

---@param disable_while_typing boolean
function DeviceHandle:set_disable_while_typing(disable_while_typing)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        disable_while_typing = disable_while_typing,
    })
end

---@param left_handed boolean
function DeviceHandle:set_left_handed(left_handed)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        left_handed = left_handed,
    })
end

---@param middle_emulation boolean
function DeviceHandle:set_middle_emulation(middle_emulation)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        middle_emulation = middle_emulation,
    })
end

---@param rotation_angle integer
function DeviceHandle:set_rotation_angle(rotation_angle)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        rotation_angle = rotation_angle,
    })
end

---@param scroll_button integer
function DeviceHandle:set_scroll_button(scroll_button)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        scroll_button = scroll_button,
    })
end

---@param scroll_button_lock boolean
function DeviceHandle:set_scroll_button_lock(scroll_button_lock)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        scroll_button_lock = scroll_button_lock,
    })
end

---@param scroll_method pinnacle.input.libinput.ScrollMethod
function DeviceHandle:set_scroll_method(scroll_method)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        scroll_method = scroll_method_values[scroll_method],
    })
end

---@param natural_scroll boolean
function DeviceHandle:set_natural_scroll(natural_scroll)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        natural_scroll = natural_scroll,
    })
end

---@param tap_button_map pinnacle.input.libinput.TapButtonMap
function DeviceHandle:set_tap_button_map(tap_button_map)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        tap_button_map = tap_button_map_values[tap_button_map],
    })
end

---@param tap_drag boolean
function DeviceHandle:set_tap_drag(tap_drag)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        tap_drag = tap_drag,
    })
end

---@param tap_drag_lock boolean
function DeviceHandle:set_tap_drag_lock(tap_drag_lock)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        tap_drag_lock = tap_drag_lock,
    })
end

---@param tap boolean
function DeviceHandle:set_tap(tap)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        tap = tap,
    })
end

---@param send_events_mode pinnacle.input.libinput.SendEventsMode
function DeviceHandle:set_send_events_mode(send_events_mode)
    local _, err = client:unary_request(input_service.SetDeviceLibinputSetting, {
        device_sysname = self.sysname,
        send_events_mode = send_events_mode_values[send_events_mode],
    })
end

---Gets all connected input devices.
---
---@return pinnacle.input.libinput.DeviceHandle[]
function libinput.get_devices()
    local response, err = client:unary_request(input_service.GetDevices, {})

    if err then
        log:error(err)
        return {}
    end

    ---@cast response pinnacle.input.v1.GetDevicesResponse

    local devices = {}
    for _, sysname in ipairs(response.device_sysnames or {}) do
        local dev = libinput.new_device(sysname)
        table.insert(devices, dev)
    end

    return devices
end

---Runs a function for every currently connected device as well as
---all devices that will be connected in the future.
---
---@param for_each fun(device: pinnacle.input.libinput.DeviceHandle)
function libinput.for_each_device(for_each)
    for _, device in ipairs(libinput.get_devices()) do
        for_each(device)
    end

    require("pinnacle.input").connect_signal({
        device_added = function(device)
            for_each(device)
        end,
    })
end

---@return pinnacle.input.libinput.DeviceHandle
---@private
function libinput.new_device(sysname)
    local device = { sysname = sysname }
    setmetatable(device, { __index = DeviceHandle })
    return device
end

return libinput
