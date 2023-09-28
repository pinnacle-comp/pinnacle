-- SPDX-License-Identifier: GPL-3.0-or-later

---@meta _

---@class LibinputSetting
---@field AccelProfile (AccelProfile)?
---@field AccelSpeed float?
---@field CalibrationMatrix float[]?
---@field ClickMethod (ClickMethod)?
---@field DisableWhileTypingEnabled boolean?
---@field LeftHanded boolean?
---@field MiddleEmulationEnabled boolean?
---@field RotationAngle integer? A u32
---@field ScrollMethod (ScrollMethod)?
---@field NaturalScrollEnabled boolean?
---@field ScrollButton integer? A u32
---@field TapButtonMap TapButtonMap?
---@field TapDragEnabled boolean?
---@field TapDragLockEnabled boolean?
---@field TapEnabled boolean?

---@alias AccelProfile
---| "Flat" # Flat pointer acceleration.
---| "Adaptive" Adaptive pointer acceleration. This is the default for most devices.

---@alias ClickMethod
---| "ButtonAreas" # Use software-button areas to generate button events.
---| "Clickfinger" # The number of fingers decides which button press to generate.

---@alias ScrollMethod
---| "NoScroll" # Never send scroll events.
---| "TwoFinger" # Send scroll events when two fingers are logically down on the device.
---| "Edge" # Send scroll events when a finger moves along the bottom or right edge of a device.
---| "OnButtonDown" # Send scroll events when a button is down and the device moves along a scroll-capable axis.

---@alias TapButtonMap
---| "LeftRightMiddle" # 1/2/3 finger tap is mapped to left/right/middle click.
---| "LeftMiddleRight" # 1/2/3 finger tap is mapped to left/middle/right click.

---Configuration options for libinput.
---
---Here, you can configure how input devices like your mouse and touchpad function.
---@class Libinput
local libinput = {}

---Set the acceleration profile.
---@param profile AccelProfile
function libinput.set_accel_profile(profile)
    SendMsg({
        SetLibinputSetting = {
            AccelProfile = profile,
        },
    })
end

---Set the acceleration speed.
---@param speed float The speed from -1 to 1.
function libinput.set_accel_speed(speed)
    SendMsg({
        SetLibinputSetting = {
            AccelSpeed = speed,
        },
    })
end

---Set the calibration matrix.
---@param matrix float[] A 6-element float array.
function libinput.set_calibration_matrix(matrix)
    if #matrix ~= 6 then
        return
    end

    SendMsg({
        SetLibinputSetting = {
            CalibrationMatrix = matrix,
        },
    })
end

---Set the click method.
---
---The click method defines when to generate software-emulated buttons, usually on a device
---that does not have a specific physical button available.
---@param method ClickMethod
function libinput.set_click_method(method)
    SendMsg({
        SetLibinputSetting = {
            ClickMethod = method,
        },
    })
end

---Set whether or not the device will be disabled while typing.
---@param enabled boolean
function libinput.set_disable_while_typing_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            DisableWhileTypingEnabled = enabled,
        },
    })
end

---Set device left-handedness.
---@param enabled boolean
function libinput.set_left_handed(enabled)
    SendMsg({
        SetLibinputSetting = {
            LeftHanded = enabled,
        },
    })
end

---Set whether or not the middle click can be emulated.
---@param enabled boolean
function libinput.set_middle_emulation_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            MiddleEmulationEnabled = enabled,
        },
    })
end

---Set the rotation angle of a device.
---@param angle integer An integer in the range [0, 360].
function libinput.set_rotation_angle(angle)
    SendMsg({
        SetLibinputSetting = {
            RotationAngle = angle,
        },
    })
end

---Set the scroll method.
---@param method ScrollMethod
function libinput.set_scroll_method(method)
    SendMsg({
        SetLibinputSetting = {
            ScrollMethod = method,
        },
    })
end

---Set whether or not natural scroll is enabled.
---
---This reverses the direction of scrolling and is mainly used with touchpads.
---@param enabled boolean
function libinput.set_natural_scroll_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            NaturalScrollEnabled = enabled,
        },
    })
end

---Set the scroll button.
---@param button MouseButton
function libinput.set_scroll_button(button)
    SendMsg({
        SetLibinputSetting = {
            ScrollButton = button,
        },
    })
end

---Set the tap button map.
---
---This determines whether taps with 2 and 3 fingers register as right and middle clicks or the reverse.
---@param map TapButtonMap
function libinput.set_tap_button_map(map)
    SendMsg({
        SetLibinputSetting = {
            TapButtonMap = map,
        },
    })
end

---Set whether or not tap-to-click is enabled.
---@param enabled boolean
function libinput.set_tap_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            TapEnabled = enabled,
        },
    })
end

---Set whether or not tap-and-drag is enabled.
---
---When enabled, a single-finger tap immediately followed by a finger down results in
---a button down event, and subsequent finger motion thus triggers a drag. The button is released on finger up.
---@param enabled boolean
function libinput.set_tap_drag_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            TapDragEnabled = enabled,
        },
    })
end

---Set whether or not tap drag lock is enabled.
---
---When enabled, a finger may be lifted and put back on the touchpad within a timeout and the drag process
---continues. When disabled, lifting the finger during a tap-and-drag will immediately stop the drag.
---@param enabled boolean
function libinput.set_tap_drag_lock_enabled(enabled)
    SendMsg({
        SetLibinputSetting = {
            TapDragLockEnabled = enabled,
        },
    })
end

return libinput
