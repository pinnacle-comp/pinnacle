use std::{collections::HashMap, path::PathBuf};

use smithay::{
    backend::renderer::{test::DummyRenderer, ImportMemWl},
    output::{Output, Subpixel},
    reexports::{
        calloop::EventLoop,
        wayland_server::{Client, Display},
    },
    utils::Transform,
};

use crate::state::State;

use super::{dummy::Dummy, Backend};

#[derive(Default)]
pub struct Wlcs {
    pub clients: HashMap<i32, Client>,
}

impl Backend {
    pub fn wlcs_mut(&mut self) -> &mut Wlcs {
        let Backend::Dummy(dummy) = self else { unreachable!() };
        &mut dummy.wlcs_state
    }
}

pub fn setup_wlcs_dummy(
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
        model: "Dummy Window".to_string(),
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
    let shm_formats = renderer.shm_formats();

    let backend = Dummy {
        renderer,
        wlcs_state: Wlcs::default(),
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

    state.shm_state.update_formats(shm_formats);

    state.space.map_output(&output, (0, 0));

    Ok((state, event_loop))
}
