#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccelProfile {
    Flat = 1,
    Adaptive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickMethod {
    ButtonAreas = 1,
    Clickfinger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollMethod {
    NoScroll = 1,
    TwoFinger,
    Edge,
    OnButtonDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TapButtonMap {
    LeftRightMiddle,
    LeftMiddleRight,
}

pub enum LibinputSetting {
    AccelProfile(AccelProfile),
    AccelSpeed(f64),
    CalibrationMatrix([f32; 6]),
    ClickMethod(ClickMethod),
    DisableWhileTyping(bool),
    LeftHanded(bool),
    MiddleEmulation(bool),
    RotationAngle(u32),
    ScrollButton(u32),
    ScrollButtonLock(u32),
    ScrollMethod(ScrollMethod),
    NaturalScroll(bool),
    TapButtonMap(TapButtonMap),
    TapDrag(bool),
    TapDragLock(bool),
    Tap(bool),
}
