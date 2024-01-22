use smithay::backend::{input::InputEvent, libinput::LibinputInputBackend};

use crate::state::State;

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

        for setting in self.input_state.libinput_settings.values() {
            setting(&mut device);
        }

        self.input_state.libinput_devices.push(device);
    }
}
