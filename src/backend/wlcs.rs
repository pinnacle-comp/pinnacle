use std::{collections::HashMap, path::Path};

use smithay::{
    backend::renderer::{test::DummyRenderer, ImportMemWl},
    output::{Output, Subpixel},
    reexports::{
        calloop::{self, EventLoop},
        wayland_server::{Client, Display},
    },
    utils::Transform,
};
use tracing::debug;

use crate::{
    state::{State, WithState},
    tag::TagId,
};

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

pub fn setup_wlcs_dummy() -> anyhow::Result<(State, EventLoop<'static, State>)> {
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
        false,
        None,
    )?;

    state.output_focus_stack.set_focus(output.clone());

    state.shm_state.update_formats(shm_formats);

    state.space.map_output(&output, (0, 0));

    Ok((state, event_loop))
}

impl State {
    pub fn start_wlcs_config<F>(
        &mut self,
        socket_dir: &Path,
        run_config: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce() -> () + Send + 'static,
    {
        // Clear state
        debug!("Clearing tags");
        for output in self.space.outputs() {
            output.with_state_mut(|state| state.tags.clear());
        }

        TagId::reset();

        debug!("Clearing input state");

        self.input_state.clear();

        self.config.clear(&self.loop_handle);

        self.signal_state.clear();

        self.input_state.reload_keybind = None;
        self.input_state.kill_keybind = None;

        if self.grpc_server_join_handle.is_none() {
            self.start_grpc_server(socket_dir)?;
        }

        let (pinger, ping_source) = calloop::ping::make_ping()?;

        let token = self
            .loop_handle
            .insert_source(ping_source, move |_, _, _state| {})?;

        std::thread::spawn(move || {
            run_config();
            pinger.ping();
        });

        self.config.config_reload_on_crash_token = Some(token);

        Ok(())
    }
}
