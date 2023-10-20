use crate::{msg::Msg, send_msg};

/// Libinput settings.
///
/// Here you can set things like pointer acceleration.
#[derive(Clone, Copy)]
pub struct Libinput;

impl Libinput {
    /// Set a libinput setting.
    ///
    /// This takes a [`LibinputSetting`] containing what you want set.
    pub fn set(&self, setting: LibinputSetting) {
        let msg = Msg::SetLibinputSetting(setting);
        send_msg(msg).unwrap();
    }
}

/// The acceleration profile.
#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum AccelProfile {
    /// Flat pointer acceleration.
    Flat,
    /// Adaptive pointer acceleration.
    ///
    /// This is the default for most devices.
    Adaptive,
}

/// The click method for a touchpad.
#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ClickMethod {
    /// Use software-button areas to generate button events.
    ButtonAreas,
    /// The number of fingers decides which button press to generate.
    Clickfinger,
}

/// The scroll method for a touchpad.
#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ScrollMethod {
    /// Never send scroll events.
    NoScroll,
    /// Send scroll events when two fingers are logically down on the device.
    TwoFinger,
    /// Send scroll events when a finger moves along the bottom or right edge of a device.
    Edge,
    /// Send scroll events when a button is down and the device moves along a scroll-capable axis.
    OnButtonDown,
}

/// The mapping between finger count and button event for a touchpad.
#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum TapButtonMap {
    /// 1/2/3 finger tap is mapped to left/right/middle click.
    LeftRightMiddle,
    /// 1/2/3 finger tap is mapped to left/middle/right click.
    LeftMiddleRight,
}

/// Libinput settings.
#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum LibinputSetting {
    /// Set the acceleration profile.
    AccelProfile(AccelProfile),
    /// Set the acceleration speed.
    ///
    /// This should be a float from -1.0 to 1.0.
    AccelSpeed(f64),
    /// Set the calibration matrix.
    CalibrationMatrix([f32; 6]),
    /// Set the click method.
    ///
    /// The click method defines when to generate software-emulated buttons, usually on a device
    /// that does not have a specific physical button available.
    ClickMethod(ClickMethod),
    /// Set whether or not the device will be disabled while typing.
    DisableWhileTypingEnabled(bool),
    /// Set device left-handedness.
    LeftHanded(bool),
    /// Set whether or not the middle click can be emulated.
    MiddleEmulationEnabled(bool),
    /// Set the rotation angle of a device.
    RotationAngle(u32),
    /// Set the scroll method.
    ScrollMethod(ScrollMethod),
    /// Set whether or not natural scroll is enabled.
    ///
    /// This reverses the direction of scrolling and is mainly used with touchpads.
    NaturalScrollEnabled(bool),
    /// Set the scroll button.
    ScrollButton(u32),
    /// Set the tap button map,
    ///
    /// This determines whether taps with 2 and 3 fingers register as right and middle clicks or
    /// the reverse.
    TapButtonMap(TapButtonMap),
    /// Set whether or not tap-and-drag is enabled.
    ///
    /// When enabled, a single-finger tap immediately followed by a finger down results in
    /// a button down event, and subsequent finger motion thus triggers a drag.
    /// The button is released on finger up.
    TapDragEnabled(bool),
    /// Set whether or not tap drag lock is enabled.
    ///
    /// When enabled, a finger may be lifted and put back on the touchpad within a timeout and the drag process
    /// continues. When disabled, lifting the finger during a tap-and-drag will immediately stop the drag.
    TapDragLockEnabled(bool),
    /// Set whether or not tap-to-click is enabled.
    TapEnabled(bool),
}
