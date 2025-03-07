// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Types for libinput configuration.

use pinnacle_api_defs::pinnacle::input::{
    self,
    v1::{
        set_device_libinput_setting_request::Setting, set_device_map_target_request::Target,
        GetDeviceCapabilitiesRequest, GetDeviceInfoRequest, GetDeviceTypeRequest,
        GetDevicesRequest, SetDeviceLibinputSettingRequest, SetDeviceMapTargetRequest,
    },
};

use crate::{client::Client, output::OutputHandle, signal::InputSignal, util::Rect, BlockOnTokio};

/// A pointer acceleration profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelProfile {
    /// A flat acceleration profile.
    ///
    /// Pointer motion is accelerated by a constant (device-specific) factor, depending on the current speed.
    Flat,
    /// An adaptive acceleration profile.
    ///
    /// Pointer acceleration depends on the input speed. This is the default profile for most devices.
    Adaptive,
}

impl From<AccelProfile> for input::v1::AccelProfile {
    fn from(value: AccelProfile) -> Self {
        match value {
            AccelProfile::Flat => input::v1::AccelProfile::Flat,
            AccelProfile::Adaptive => input::v1::AccelProfile::Adaptive,
        }
    }
}

/// The click method defines when to generate software-emulated buttons, usually on a device
/// that does not have a specific physical button available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickMethod {
    /// Use software-button areas to generate button events.
    ButtonAreas,
    /// The number of fingers decides which button press to generate.
    Clickfinger,
}

impl From<ClickMethod> for input::v1::ClickMethod {
    fn from(value: ClickMethod) -> Self {
        match value {
            ClickMethod::ButtonAreas => input::v1::ClickMethod::ButtonAreas,
            ClickMethod::Clickfinger => input::v1::ClickMethod::ClickFinger,
        }
    }
}

/// The scroll method of a device selects when to generate scroll axis events instead of pointer motion events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollMethod {
    /// Never send scroll events instead of pointer motion events.
    ///
    /// This has no effect on events generated by scroll wheels.
    NoScroll,
    /// Send scroll events when two fingers are logically down on the device.
    TwoFinger,
    /// Send scroll events when a finger moves along the bottom or right edge of a device.
    Edge,
    /// Send scroll events when a button is down and the device moves along a scroll-capable axis.
    OnButtonDown,
}

impl From<ScrollMethod> for input::v1::ScrollMethod {
    fn from(value: ScrollMethod) -> Self {
        match value {
            ScrollMethod::NoScroll => input::v1::ScrollMethod::NoScroll,
            ScrollMethod::TwoFinger => input::v1::ScrollMethod::TwoFinger,
            ScrollMethod::Edge => input::v1::ScrollMethod::Edge,
            ScrollMethod::OnButtonDown => input::v1::ScrollMethod::OnButtonDown,
        }
    }
}

/// Map 1/2/3 finger taps to buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TapButtonMap {
    /// 1/2/3 finger tap maps to left/right/middle
    LeftRightMiddle,
    /// 1/2/3 finger tap maps to left/middle/right
    LeftMiddleRight,
}

impl From<TapButtonMap> for input::v1::TapButtonMap {
    fn from(value: TapButtonMap) -> Self {
        match value {
            TapButtonMap::LeftRightMiddle => input::v1::TapButtonMap::LeftRightMiddle,
            TapButtonMap::LeftMiddleRight => input::v1::TapButtonMap::LeftMiddleRight,
        }
    }
}

/// A libinput send events mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SendEventsMode {
    /// Enable this device.
    Enabled,
    /// Disable this device.
    Disabled,
    /// Disable this device only when an external mouse is connected.
    DisabledOnExternalMouse,
}

impl From<SendEventsMode> for input::v1::SendEventsMode {
    fn from(value: SendEventsMode) -> Self {
        match value {
            SendEventsMode::Enabled => input::v1::SendEventsMode::Enabled,
            SendEventsMode::Disabled => input::v1::SendEventsMode::Disabled,
            SendEventsMode::DisabledOnExternalMouse => {
                input::v1::SendEventsMode::DisabledOnExternalMouse
            }
        }
    }
}

bitflags::bitflags! {
    /// A device's libinput capabilities.
    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Capability: u16 {
        /// This device has keyboard capabilities.
        const KEYBOARD = 1;
        /// This device has pointer capabilities.
        const POINTER = 1 << 1;
        /// This device has touch capabilities.
        const TOUCH = 1 << 2;
        /// This device has tablet tool capabilities.
        const TABLET_TOOL = 1 << 3;
        /// This device has tablet pad capabilities.
        const TABLET_PAD = 1 << 4;
        /// This device has gesture capabilities.
        const GESTURE = 1 << 5;
        /// This device has switch capabilities.
        const SWITCH = 1 << 6;
    }
}

/// A device's type.
///
/// Note: this uses heuristics to determine device type.
/// *This may be incorrect*. For example, a device with both pointer
/// and keyboard capabilities will be labeled as a `Mouse` when it might actually be
/// a keyboard.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum DeviceType {
    /// The device type is unknown.
    #[default]
    Unknown,
    /// This device is a touchpad.
    Touchpad,
    /// This device is a trackball.
    Trackball,
    /// This device is a trackpoint.
    Trackpoint,
    /// This device is a mouse.
    Mouse,
    /// This device is a tablet.
    Tablet,
    /// This device is a keyboard.
    Keyboard,
    /// This device is a switch.
    Switch,
}

impl DeviceType {
    /// Returns `true` if the device type is [`Unknown`].
    ///
    /// [`Unknown`]: DeviceType::Unknown
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Returns `true` if the device type is [`Touchpad`].
    ///
    /// [`Touchpad`]: DeviceType::Touchpad
    #[must_use]
    pub fn is_touchpad(&self) -> bool {
        matches!(self, Self::Touchpad)
    }

    /// Returns `true` if the device type is [`Trackball`].
    ///
    /// [`Trackball`]: DeviceType::Trackball
    #[must_use]
    pub fn is_trackball(&self) -> bool {
        matches!(self, Self::Trackball)
    }

    /// Returns `true` if the device type is [`Trackpoint`].
    ///
    /// [`Trackpoint`]: DeviceType::Trackpoint
    #[must_use]
    pub fn is_trackpoint(&self) -> bool {
        matches!(self, Self::Trackpoint)
    }

    /// Returns `true` if the device type is [`Mouse`].
    ///
    /// [`Mouse`]: DeviceType::Mouse
    #[must_use]
    pub fn is_mouse(&self) -> bool {
        matches!(self, Self::Mouse)
    }

    /// Returns `true` if the device type is [`Tablet`].
    ///
    /// [`Tablet`]: DeviceType::Tablet
    #[must_use]
    pub fn is_tablet(&self) -> bool {
        matches!(self, Self::Tablet)
    }

    /// Returns `true` if the device type is [`Keyboard`].
    ///
    /// [`Keyboard`]: DeviceType::Keyboard
    #[must_use]
    pub fn is_keyboard(&self) -> bool {
        matches!(self, Self::Keyboard)
    }

    /// Returns `true` if the device type is [`Switch`].
    ///
    /// [`Switch`]: DeviceType::Switch
    #[must_use]
    pub fn is_switch(&self) -> bool {
        matches!(self, Self::Switch)
    }
}

impl From<input::v1::DeviceType> for DeviceType {
    fn from(value: input::v1::DeviceType) -> Self {
        match value {
            input::v1::DeviceType::Unspecified => DeviceType::Unknown,
            input::v1::DeviceType::Touchpad => DeviceType::Touchpad,
            input::v1::DeviceType::Trackball => DeviceType::Trackball,
            input::v1::DeviceType::Trackpoint => DeviceType::Trackpoint,
            input::v1::DeviceType::Mouse => DeviceType::Mouse,
            input::v1::DeviceType::Tablet => DeviceType::Tablet,
            input::v1::DeviceType::Keyboard => DeviceType::Keyboard,
            input::v1::DeviceType::Switch => DeviceType::Switch,
        }
    }
}

/// A libinput device.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct DeviceHandle {
    pub(crate) sysname: String,
}

impl DeviceHandle {
    /// Gets the [capabilities][Capability] of this device.
    pub fn capabilities(&self) -> Capability {
        self.capabilities_async().block_on_tokio()
    }

    /// Async impl for [`Self::capabilities`].
    pub async fn capabilities_async(&self) -> Capability {
        let caps = Client::input()
            .get_device_capabilities(GetDeviceCapabilitiesRequest {
                device_sysname: self.sysname.clone(),
            })
            .await
            .unwrap()
            .into_inner();

        let mut capability = Capability::default();

        if caps.keyboard {
            capability |= Capability::KEYBOARD;
        }
        if caps.pointer {
            capability |= Capability::POINTER;
        }
        if caps.touch {
            capability |= Capability::TOUCH;
        }
        if caps.tablet_tool {
            capability |= Capability::TABLET_TOOL;
        }
        if caps.tablet_pad {
            capability |= Capability::TABLET_PAD;
        }
        if caps.gesture {
            capability |= Capability::GESTURE;
        }
        if caps.switch {
            capability |= Capability::SWITCH;
        }

        capability
    }

    /// Gets this device's name.
    pub fn name(&self) -> String {
        self.name_async().block_on_tokio()
    }

    /// Async impl for [`Self::name`].
    pub async fn name_async(&self) -> String {
        Client::input()
            .get_device_info(GetDeviceInfoRequest {
                device_sysname: self.sysname.clone(),
            })
            .await
            .unwrap()
            .into_inner()
            .name
    }

    /// Gets this device's product id.
    pub fn product_id(&self) -> u32 {
        self.product_id_async().block_on_tokio()
    }

    /// Async impl for [`Self::product_id`].
    pub async fn product_id_async(&self) -> u32 {
        Client::input()
            .get_device_info(GetDeviceInfoRequest {
                device_sysname: self.sysname.clone(),
            })
            .await
            .unwrap()
            .into_inner()
            .product_id
    }

    /// Gets this device's vendor id.
    pub fn vendor_id(&self) -> u32 {
        self.vendor_id_async().block_on_tokio()
    }

    /// Async impl for [`Self::vendor_id`].
    pub async fn vendor_id_async(&self) -> u32 {
        Client::input()
            .get_device_info(GetDeviceInfoRequest {
                device_sysname: self.sysname.clone(),
            })
            .await
            .unwrap()
            .into_inner()
            .vendor_id
    }

    /// Gets this device's [`DeviceType`].
    pub fn device_type(&self) -> DeviceType {
        self.device_type_async().block_on_tokio()
    }

    /// Async impl for [`Self::device_type`].
    pub async fn device_type_async(&self) -> DeviceType {
        Client::input()
            .get_device_type(GetDeviceTypeRequest {
                device_sysname: self.sysname.clone(),
            })
            .await
            .unwrap()
            .into_inner()
            .device_type()
            .into()
    }

    /// Maps the absolute input from this device to the corresponding output.
    ///
    /// This will cause touch input from this device to map proportionally
    /// to the area of an output. For example, tapping in the middle of the device
    /// will generate a tap event at the middle of the output.
    ///
    /// This only affects devices with touch capability.
    ///
    /// If you want to map the device to an arbitrary region, see [`Self::map_to_region`].
    pub fn map_to_output(&self, output: &OutputHandle) {
        Client::input()
            .set_device_map_target(SetDeviceMapTargetRequest {
                device_sysname: self.sysname.clone(),
                target: Some(Target::OutputName(output.name())),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Maps the absolute input from this device to the corresponding region
    /// in the global space.
    ///
    /// This will cause touch input from this device to map proportionally
    /// to the given region within the global space. For example, tapping in the middle of the device
    /// will generate a tap event at the middle of the region. This can be used
    /// to map a touch device to more than one output, for example.
    ///
    /// This only affects devices with touch capability.
    ///
    /// If you want to map the device to a single output, see [`Self::map_to_output`].
    pub fn map_to_region(&self, region: Rect) {
        Client::input()
            .set_device_map_target(SetDeviceMapTargetRequest {
                device_sysname: self.sysname.clone(),
                target: Some(Target::Region(region.into())),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's acceleration profile.
    pub fn set_accel_profile(&self, accel_profile: AccelProfile) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::AccelProfile(
                    input::v1::AccelProfile::from(accel_profile).into(),
                )),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's acceleration speed.
    pub fn set_accel_speed(&self, accel_speed: f64) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::AccelSpeed(accel_speed)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's calibration matrix.
    pub fn set_calibration_matrix(&self, calibration_matrix: [f32; 6]) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::CalibrationMatrix(input::v1::CalibrationMatrix {
                    matrix: calibration_matrix.to_vec(),
                })),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's click method.
    pub fn set_click_method(&self, click_method: ClickMethod) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::ClickMethod(
                    input::v1::ClickMethod::from(click_method).into(),
                )),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets whether or not this device is disabled while typing.
    pub fn set_disable_while_typing(&self, disable_while_typing: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::DisableWhileTyping(disable_while_typing)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device to left-handed or not.
    pub fn set_left_handed(&self, left_handed: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::LeftHanded(left_handed)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets whether or not middle emulation is enabled.
    pub fn set_middle_emulation(&self, middle_emulation: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::MiddleEmulation(middle_emulation)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's rotation angle.
    pub fn set_rotation_angle(&self, rotation_angle: u32) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::RotationAngle(rotation_angle)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's scroll button.
    pub fn set_scroll_button(&self, scroll_button: u32) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::ScrollButton(scroll_button)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets whether or not the scroll button locks on this device.
    pub fn set_scroll_button_lock(&self, scroll_button_lock: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::ScrollButtonLock(scroll_button_lock)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's scroll method.
    pub fn set_scroll_method(&self, scroll_method: ScrollMethod) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::ScrollMethod(
                    input::v1::ScrollMethod::from(scroll_method).into(),
                )),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Enables or disables natural scroll on this device.
    pub fn set_natural_scroll(&self, natural_scroll: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::NaturalScroll(natural_scroll)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's tap button map.
    pub fn set_tap_button_map(&self, tap_button_map: TapButtonMap) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::TapButtonMap(
                    input::v1::TapButtonMap::from(tap_button_map).into(),
                )),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Enables or disables tap dragging on this device.
    pub fn set_tap_drag(&self, tap_drag: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::TapDrag(tap_drag)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets whether or not tap dragging locks on this device.
    pub fn set_tap_drag_lock(&self, tap_drag_lock: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::TapDragLock(tap_drag_lock)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Enables or disables tap-to-click on this device.
    pub fn set_tap(&self, tap: bool) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::Tap(tap)),
            })
            .block_on_tokio()
            .unwrap();
    }

    /// Sets this device's send events mode.
    pub fn set_send_events_mode(&self, send_events_mode: SendEventsMode) {
        Client::input()
            .set_device_libinput_setting(SetDeviceLibinputSettingRequest {
                device_sysname: self.sysname.clone(),
                setting: Some(Setting::SendEventsMode(
                    input::v1::SendEventsMode::from(send_events_mode).into(),
                )),
            })
            .block_on_tokio()
            .unwrap();
    }
}

/// Gets handles to all connected input devices.
pub fn get_devices() -> impl Iterator<Item = DeviceHandle> {
    Client::input()
        .get_devices(GetDevicesRequest {})
        .block_on_tokio()
        .unwrap()
        .into_inner()
        .device_sysnames
        .into_iter()
        .map(|sysname| DeviceHandle { sysname })
}

/// Runs a closure for all current and future input devices.
///
/// This function does two things:
///   1. Runs `for_all` with all currently connected input devices, and
///   2. Runs it with all newly connected devices.
///
/// Use this function for input device setup.
pub fn for_each_device<F: FnMut(&DeviceHandle) + Send + 'static>(mut for_all: F) {
    for device in get_devices() {
        for_all(&device);
    }

    super::connect_signal(InputSignal::DeviceAdded(Box::new(for_all)));
}
