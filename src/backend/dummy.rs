use pinnacle_api_defs::pinnacle::signal::v0alpha1::{
    OutputConnectResponse, OutputDisconnectResponse,
};
use smithay::backend::renderer::test::DummyRenderer;
use smithay::backend::renderer::ImportMemWl;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::utils::{Physical, Size};

use smithay::{
    output::{Output, Subpixel},
    utils::Transform,
};

use crate::state::{Pinnacle, State, WithState};

use super::BackendData;
use super::{Backend, UninitBackend};

pub const DUMMY_OUTPUT_NAME: &str = "Dummy Window";

#[cfg(feature = "wlcs")]
#[derive(Default)]
pub struct Wlcs {
    pub clients: std::collections::HashMap<i32, smithay::reexports::wayland_server::Client>,
}

pub struct Dummy {
    pub renderer: DummyRenderer,
    // pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
    #[cfg(feature = "wlcs")]
    pub wlcs_state: Wlcs,
}

impl Backend {
    #[cfg(feature = "wlcs")]
    pub fn wlcs_mut(&mut self) -> &mut Wlcs {
        let Backend::Dummy(dummy) = self else {
            unreachable!(r#"feature gated by "wlcs""#)
        };
        &mut dummy.wlcs_state
    }
}

impl BackendData for Dummy {
    fn seat_name(&self) -> String {
        "Dummy".to_string()
    }

    fn reset_buffers(&mut self, _output: &Output) {}

    fn early_import(&mut self, _surface: &WlSurface) {}
}

impl Dummy {
    pub(crate) fn try_new(display_handle: DisplayHandle) -> UninitBackend<Dummy> {
        let mode = smithay::output::Mode {
            size: (1920, 1080).into(),
            refresh: 60_000,
        };

        let physical_properties = smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Pinnacle".to_string(),
            model: "Dummy Window".to_string(),
        };

        let output = Output::new(DUMMY_OUTPUT_NAME.to_string(), physical_properties);

        output.change_current_state(
            Some(mode),
            Some(Transform::Flipped180),
            None,
            Some((0, 0).into()),
        );

        output.set_preferred(mode);
        output.with_state_mut(|state| state.modes = vec![mode]);

        let renderer = DummyRenderer::new();

        let dummy = Dummy {
            renderer,
            // dmabuf_state,
            #[cfg(feature = "wlcs")]
            wlcs_state: Wlcs::default(),
        };

        UninitBackend {
            seat_name: dummy.seat_name(),
            init: Box::new(move |pinnacle| {
                output.create_global::<State>(&display_handle);

                pinnacle.output_focus_stack.set_focus(output.clone());

                pinnacle
                    .shm_state
                    .update_formats(dummy.renderer.shm_formats());

                pinnacle.space.map_output(&output, (0, 0));

                Ok(dummy)
            }),
        }
    }
}

impl Pinnacle {
    pub fn new_output(&mut self, name: impl std::fmt::Display, size: Size<i32, Physical>) {
        let mode = smithay::output::Mode {
            size,
            refresh: 144_000,
        };

        let physical_properties = smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Pinnacle".to_string(),
            model: "Dummy Output".to_string(),
        };

        let output = Output::new(name.to_string(), physical_properties);

        output.change_current_state(Some(mode), None, None, Some((0, 0).into()));

        output.set_preferred(mode);
        output.with_state_mut(|state| state.modes = vec![mode]);

        output.create_global::<State>(&self.display_handle);

        self.space.map_output(&output, (0, 0));

        self.signal_state.output_connect.signal(|buf| {
            buf.push_back(OutputConnectResponse {
                output_name: Some(output.name()),
            });
        });
    }

    pub fn remove_output(&mut self, output: &Output) {
        self.space.unmap_output(output);

        self.signal_state.output_disconnect.signal(|buffer| {
            buffer.push_back(OutputDisconnectResponse {
                output_name: Some(output.name()),
            })
        });
    }
}
