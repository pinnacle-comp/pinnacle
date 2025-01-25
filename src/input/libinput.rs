use std::collections::HashSet;

use smithay::reexports::input::Device;

#[derive(Debug, Default)]
pub struct LibinputState {
    pub devices: HashSet<Device>,
}

// This may not be right, idk if a device can be both a trackball and
// trackpoint for instance. And I know for a fact that there are devices
// with both the pointer and keyboard capability.
pub enum DeviceType {
    Unknown,
    Touchpad,
    Trackball,
    Trackpoint,
    Mouse,
    Tablet,
    Keyboard,
    Switch,
}

// Logic from https://github.com/YaLTeR/niri/blob/b3c6f0e661878c7ab4f3c84c480ae61a5de5d3b3/src/input/mod.rs#L3013
pub fn device_type(device: &Device) -> DeviceType {
    let is_touchpad = device.config_tap_finger_count() > 0;

    let mut is_trackball = false;
    let mut is_trackpoint = false;
    if let Some(udev_device) = unsafe { device.udev_device() } {
        is_trackball = udev_device.property_value("ID_INPUT_TRACKBALL").is_some();

        is_trackpoint = udev_device
            .property_value("ID_INPUT_POINTINGSTICK")
            .is_some();
    }

    let is_mouse = device.has_capability(smithay::reexports::input::DeviceCapability::Pointer);
    let is_tablet = device.has_capability(smithay::reexports::input::DeviceCapability::TabletTool); // yo I should get a dirt cheap drawing tablet to test this
    let is_switch = device.has_capability(smithay::reexports::input::DeviceCapability::Switch);
    let is_keyboard = device.has_capability(smithay::reexports::input::DeviceCapability::Keyboard);

    if is_mouse && !is_trackball && !is_trackpoint && !is_touchpad {
        DeviceType::Mouse
    } else if is_touchpad {
        DeviceType::Touchpad
    } else if is_trackball {
        DeviceType::Trackball
    } else if is_trackpoint {
        DeviceType::Trackpoint
    } else if is_tablet {
        DeviceType::Tablet
    } else if is_switch {
        DeviceType::Switch
    } else if is_keyboard {
        DeviceType::Keyboard
    } else {
        DeviceType::Unknown
    }
}
