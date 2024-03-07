use smithay::backend::renderer::test::DummyRenderer;
use smithay::backend::renderer::ImportMemWl;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use std::ffi::OsString;
use std::path::PathBuf;

use smithay::{
    output::{Output, Subpixel},
    reexports::{calloop::EventLoop, wayland_server::Display},
    utils::Transform,
};

use crate::state::State;

use super::Backend;
use super::BackendData;

pub struct Dummy {
    pub renderer: DummyRenderer,
    // pub dmabuf_state: (DmabufState, DmabufGlobal, Option<DmabufFeedback>),
}

impl Backend {
    fn dummy_mut(&mut self) -> &Dummy {
        let Backend::Dummy(dummy) = self else { unreachable!() };
        dummy
    }
}

impl BackendData for Dummy {
    fn seat_name(&self) -> String {
        "Dummy".to_string()
    }

    fn reset_buffers(&mut self, _output: &Output) {}

    fn early_import(&mut self, _surface: &WlSurface) {}
}

pub fn setup_dummy(
    no_config: bool,
    config_dir: Option<PathBuf>,
) -> anyhow::Result<(State, EventLoop<'static, State>)> {
    let event_loop: EventLoop<State> = EventLoop::try_new()?;

    let display: Display<State> = Display::new()?;
    let display_handle = display.handle();

    let loop_handle = event_loop.handle();

    let mode = smithay::output::Mode {
        size: (1920, 1080).into(),
        refresh: 60_000,
    };

    let physical_properties = smithay::output::PhysicalProperties {
        size: (0, 0).into(),
        subpixel: Subpixel::Unknown,
        make: "Pinnacle".to_string(),
        model: "Winit Window".to_string(),
    };

    let output = Output::new("Pinnacle Window".to_string(), physical_properties);

    output.create_global::<State>(&display_handle);

    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );

    output.set_preferred(mode);

    let renderer = DummyRenderer::new();

    // let dmabuf_state = {
    //     let dmabuf_formats = renderer.dmabuf_formats().collect::<Vec<_>>();
    //     let mut dmabuf_state = DmabufState::new();
    //     let dmabuf_global = dmabuf_state.create_global::<State>(&display_handle, dmabuf_formats);
    //     (dmabuf_state, dmabuf_global, None)
    // };

    let backend = Dummy {
        renderer,
        // dmabuf_state,
    };

    let mut state = State::init(
        super::Backend::Dummy(backend),
        display,
        event_loop.get_signal(),
        loop_handle,
        no_config,
        config_dir,
    )?;

    state.output_focus_stack.set_focus(output.clone());

    let dummy = state.backend.dummy_mut();

    state.shm_state.update_formats(dummy.renderer.shm_formats());

    state.space.map_output(&output, (0, 0));

    if let Err(err) = state.xwayland.start(
        state.loop_handle.clone(),
        None,
        std::iter::empty::<(OsString, OsString)>(),
        true,
        |_| {},
    ) {
        tracing::error!("Failed to start XWayland: {err}");
    }

    Ok((state, event_loop))
}
