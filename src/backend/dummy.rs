use smithay::backend::renderer::test::DummyRenderer;
use smithay::backend::renderer::ImportMemWl;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::utils::{Logical, Physical, Point, Size};

use smithay::{
    output::{Output, Subpixel},
    utils::Transform,
};

use crate::api::signal::Signal;
use crate::output::OutputMode;
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

    fn set_output_mode(&mut self, output: &Output, mode: OutputMode) {
        output.change_current_state(Some(mode.into()), None, None, None);
    }
}

impl Dummy {
    pub(crate) fn try_new() -> UninitBackend<Dummy> {
        let dummy = Dummy {
            renderer: DummyRenderer,
            // dmabuf_state,
            #[cfg(feature = "wlcs")]
            wlcs_state: Wlcs::default(),
        };

        UninitBackend {
            seat_name: dummy.seat_name(),
            init: Box::new(move |pinnacle| {
                pinnacle
                    .shm_state
                    .update_formats(dummy.renderer.shm_formats());

                Ok(dummy)
            }),
        }
    }

    pub(super) fn set_output_powered(&self, output: &Output, powered: bool) {
        output.with_state_mut(|state| state.powered = powered);
    }
}

impl Pinnacle {
    pub fn new_output(
        &mut self,
        name: impl std::fmt::Display,
        make: impl std::fmt::Display,
        model: impl std::fmt::Display,
        loc: Point<i32, Logical>,
        size: Size<i32, Physical>,
        refresh: i32,
        scale: f64,
        transform: Transform,
    ) -> Output {
        let mode = smithay::output::Mode { size, refresh };

        let physical_properties = smithay::output::PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: make.to_string(),
            model: model.to_string(),
        };

        let output = Output::new(name.to_string(), physical_properties);

        output.change_current_state(
            Some(mode),
            Some(transform),
            Some(smithay::output::Scale::Fractional(scale)),
            Some(loc),
        );

        output.set_preferred(mode);
        output.with_state_mut(|state| state.modes = vec![mode]);

        let global = output.create_global::<State>(&self.display_handle);

        self.outputs.insert(output.clone(), Some(global));

        self.space.map_output(&output, loc);

        self.signal_state.output_connect.signal(&output);

        self.output_focus_stack.set_focus(output.clone());

        output
    }
}
