use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;

#[derive(Default, Debug, Clone)]
pub struct OutputState {
    pub output: Option<WlOutput>,
}

impl OutputState {
    pub fn enter(&mut self, output: WlOutput) {
        self.output = Some(output);
    }

    pub fn leave(&mut self, output: WlOutput) {
        if self.output == Some(output) {
            self.output = None;
        }
    }
}

pub mod operation {
    use iced_wgpu::core::widget::Operation;
    use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;

    pub fn enter_output(handle: WlOutput) -> impl Operation {
        struct EnterOutput {
            handle: WlOutput,
        }

        impl Operation for EnterOutput {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::OutputState>() else {
                    return;
                };

                state.enter(self.handle.clone());
            }
        }

        EnterOutput { handle }
    }

    pub fn leave_output(handle: WlOutput) -> impl Operation {
        struct LeaveOutput {
            handle: WlOutput,
        }

        impl Operation for LeaveOutput {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::OutputState>() else {
                    return;
                };

                state.leave(self.handle.clone());
            }
        }

        LeaveOutput { handle }
    }
}
