use smithay::{
    backend::{input::InputEvent, libinput::LibinputInputBackend},
    reexports::input::{self, AccelProfile, ClickMethod, ScrollMethod, TapButtonMap},
};

use crate::state::State;

#[derive(Debug, serde::Deserialize)]
#[serde(remote = "AccelProfile")]
enum AccelProfileDef {
    Flat,
    Adaptive,
}

#[derive(Debug, serde::Deserialize)]
#[serde(remote = "ClickMethod")]
enum ClickMethodDef {
    ButtonAreas,
    Clickfinger,
}

#[derive(Debug, serde::Deserialize)]
#[serde(remote = "ScrollMethod")]
enum ScrollMethodDef {
    NoScroll,
    TwoFinger,
    Edge,
    OnButtonDown,
}

#[derive(Debug, serde::Deserialize)]
#[serde(remote = "TapButtonMap")]
enum TapButtonMapDef {
    LeftRightMiddle,
    LeftMiddleRight,
}

#[derive(Debug, PartialEq, Copy, Clone, serde::Deserialize)]
pub enum LibinputSetting {
    #[serde(with = "AccelProfileDef")]
    AccelProfile(AccelProfile),
    AccelSpeed(f64),
    CalibrationMatrix([f32; 6]),
    #[serde(with = "ClickMethodDef")]
    ClickMethod(ClickMethod),
    DisableWhileTypingEnabled(bool),
    LeftHanded(bool),
    MiddleEmulationEnabled(bool),
    RotationAngle(u32),
    #[serde(with = "ScrollMethodDef")]
    ScrollMethod(ScrollMethod),
    NaturalScrollEnabled(bool),
    ScrollButton(u32),
    #[serde(with = "TapButtonMapDef")]
    TapButtonMap(TapButtonMap),
    TapDragEnabled(bool),
    TapDragLockEnabled(bool),
    TapEnabled(bool),
}

impl LibinputSetting {
    pub fn apply_to_device(&self, device: &mut input::Device) {
        let _ = match self {
            LibinputSetting::AccelProfile(profile) => device.config_accel_set_profile(*profile),
            LibinputSetting::AccelSpeed(speed) => device.config_accel_set_speed(*speed),
            LibinputSetting::CalibrationMatrix(matrix) => {
                device.config_calibration_set_matrix(*matrix)
            }
            LibinputSetting::ClickMethod(method) => device.config_click_set_method(*method),
            LibinputSetting::DisableWhileTypingEnabled(enabled) => {
                device.config_dwt_set_enabled(*enabled)
            }
            LibinputSetting::LeftHanded(enabled) => device.config_left_handed_set(*enabled),
            LibinputSetting::MiddleEmulationEnabled(enabled) => {
                device.config_middle_emulation_set_enabled(*enabled)
            }
            LibinputSetting::RotationAngle(angle) => device.config_rotation_set_angle(*angle),
            LibinputSetting::ScrollMethod(method) => device.config_scroll_set_method(*method),
            LibinputSetting::NaturalScrollEnabled(enabled) => {
                device.config_scroll_set_natural_scroll_enabled(*enabled)
            }
            LibinputSetting::ScrollButton(button) => device.config_scroll_set_button(*button),
            LibinputSetting::TapButtonMap(map) => device.config_tap_set_button_map(*map),
            LibinputSetting::TapDragEnabled(enabled) => {
                device.config_tap_set_drag_enabled(*enabled)
            }
            LibinputSetting::TapDragLockEnabled(enabled) => {
                device.config_tap_set_drag_lock_enabled(*enabled)
            }
            LibinputSetting::TapEnabled(enabled) => device.config_tap_set_enabled(*enabled),
        };
    }
}

// We want to completely replace old settings, so we hash only the discriminant.
impl std::hash::Hash for LibinputSetting {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl State {
    /// Apply current libinput settings to new devices.
    pub fn apply_libinput_settings(&mut self, event: &InputEvent<LibinputInputBackend>) {
        let mut device = match event {
            InputEvent::DeviceAdded { device } => device.clone(),
            InputEvent::DeviceRemoved { device } => {
                self.input_state
                    .libinput_devices
                    .retain(|dev| dev != device);
                return;
            }
            _ => return,
        };

        if self.input_state.libinput_devices.contains(&device) {
            return;
        }

        for setting in self.input_state.libinput_settings.iter() {
            setting.apply_to_device(&mut device);
        }
        for setting in self.input_state.grpc_libinput_settings.values() {
            setting(&mut device);
        }

        self.input_state.libinput_devices.push(device);
    }
}
