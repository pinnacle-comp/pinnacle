use crate::{msg::Msg, send_msg};

#[derive(Clone, Copy)]
pub struct Libinput;

impl Libinput {
    pub fn set(&self, setting: LibinputSetting) {
        let msg = Msg::SetLibinputSetting(setting);
        send_msg(msg).unwrap();
    }
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum AccelProfile {
    Flat,
    Adaptive,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ClickMethod {
    ButtonAreas,
    Clickfinger,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum ScrollMethod {
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum TapButtonMap {
    LeftRightMiddle,
    LeftMiddleRight,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Serialize)]
pub enum LibinputSetting {
    AccelProfile(AccelProfile),
    AccelSpeed(f64),
    CalibrationMatrix([f32; 6]),
    ClickMethod(ClickMethod),
    DisableWhileTypingEnabled(bool),
    LeftHanded(bool),
    MiddleEmulationEnabled(bool),
    RotationAngle(u32),
    ScrollMethod(ScrollMethod),
    NaturalScrollEnabled(bool),
    ScrollButton(u32),
    TapButtonMap(TapButtonMap),
    TapDragEnabled(bool),
    TapDragLockEnabled(bool),
    TapEnabled(bool),
}
