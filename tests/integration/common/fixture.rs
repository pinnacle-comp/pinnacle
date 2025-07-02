use std::{
    os::fd::AsFd,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, Mutex, MutexGuard,
    },
    time::Duration,
};

use pinnacle::state::{ClientState, Pinnacle};
use smithay::{
    output::Output,
    reexports::calloop::{generic::Generic, EventLoop, Interest, Mode, PostAction},
    utils::{Logical, Rectangle, Transform},
};
use tracing::debug;
use wayland_client::protocol::wl_surface::WlSurface;

use super::{
    client::{Client, ClientId, Window},
    server::Server,
};

static TEST_MUTEX: Mutex<()> = Mutex::new(());

pub struct Fixture {
    event_loop: EventLoop<'static, State>,
    state: State,
    _test_guard: MutexGuard<'static, ()>,
}

struct State {
    server: Server,
    clients: Vec<Client>,
}

static OUTPUT_COUNTER: AtomicU32 = AtomicU32::new(0);

impl Fixture {
    pub fn new() -> Self {
        Self::new_inner(false)
    }

    pub fn new_with_socket() -> Self {
        Self::new_inner(true)
    }

    pub fn new_inner(create_socket: bool) -> Self {
        let _test_guard = TEST_MUTEX.lock().unwrap_or_else(|guard| {
            TEST_MUTEX.clear_poison();
            guard.into_inner()
        });

        let state = State {
            server: Server::new(create_socket),
            clients: Vec::new(),
        };

        let event_loop = EventLoop::try_new().unwrap();

        // Fold the server's event loop into the fixture's
        let fd = state
            .server
            .event_loop
            .as_fd()
            .try_clone_to_owned()
            .unwrap();
        let source = Generic::new(fd, Interest::READ, Mode::Level);
        event_loop
            .handle()
            .insert_source(source, |_, _, state: &mut State| {
                state.server.dispatch();
                Ok(PostAction::Continue)
            })
            .unwrap();

        Self {
            event_loop,
            state,
            _test_guard,
        }
    }

    pub fn runtime_handle(&self) -> tokio::runtime::Handle {
        self.state.server.runtime.handle().clone()
    }

    pub fn add_client(&mut self) -> ClientId {
        let (sock1, sock2) = std::os::unix::net::UnixStream::pair().unwrap();

        let client = Client::new(sock2);
        let id = client.id();

        // Fold the client's event loop into the fixture's
        self.pinnacle()
            .display_handle
            .insert_client(sock1, Arc::new(ClientState::default()))
            .unwrap();
        let fd = client.event_loop_fd();
        let source = Generic::new(fd, Interest::READ, Mode::Level);
        self.event_loop
            .handle()
            .insert_source(source, move |_, _, state: &mut State| {
                state.client(id).dispatch();
                Ok(PostAction::Continue)
            })
            .unwrap();

        self.state.clients.push(client);
        self.roundtrip(id);
        id
    }

    pub fn add_output(&mut self, geo: Rectangle<i32, Logical>) -> Output {
        let name = format!(
            "pinnacle-{}",
            OUTPUT_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        self.pinnacle().new_output(
            name,
            "",
            "",
            geo.loc,
            geo.size.to_physical(1),
            60000,
            1.0,
            Transform::Normal,
        )
    }

    pub fn state(&mut self) -> &mut pinnacle::state::State {
        &mut self.state.server.state
    }

    pub fn pinnacle(&mut self) -> &mut Pinnacle {
        &mut self.state().pinnacle
    }

    pub fn dispatch(&mut self) {
        self.event_loop
            .dispatch(Duration::ZERO, &mut self.state)
            .unwrap();
    }

    pub fn dispatch_until<F>(&mut self, mut until: F)
    where
        F: FnMut(&mut Self) -> bool,
    {
        while !until(self) {
            self.dispatch();
        }
    }

    /// Spawns a blocking API call and dispatches the event loop until it is finished.
    #[track_caller]
    pub fn spawn_blocking<F, T>(&mut self, spawn: F) -> T
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.runtime_handle();
        let _guard = handle.enter();
        let join = handle.spawn_blocking(spawn);
        self.dispatch_until(|_| join.is_finished());

        match self.runtime_handle().block_on(join) {
            Ok(ret) => ret,
            Err(err) => {
                panic!("rust panicked: {err}");
            }
        }
    }

    #[track_caller]
    pub fn spawn_lua_blocking(&mut self, code: impl ToString) {
        let code = code.to_string();
        let join = std::thread::spawn(move || {
            let lua = crate::common::new_lua();
            let task = lua.load(format!(
                "
                Pinnacle.run(function()
                    local run = function()
                        {code}
                    end

                    local success, err = pcall(run)

                    if not success then
                        error(err)
                    end
                end)
"
            ));

            if let Err(err) = task.exec() {
                panic!("lua panicked: {err}");
            }
        });
        self.dispatch_until(|_| join.is_finished());
        join.join().unwrap();
    }

    pub fn roundtrip(&mut self, id: ClientId) {
        let client = self.client(id);
        let wait = client.send_sync();
        while !wait.load(Ordering::Relaxed) {
            self.dispatch();
        }
        debug!(client = ?id, "roundtripped");
    }

    pub fn spawn_window_with<F>(&mut self, id: ClientId, mut pre_initial_commit: F) -> WlSurface
    where
        F: FnMut(&mut Window),
    {
        // Add a window
        let window = self.client(id).create_window();
        pre_initial_commit(window);
        window.commit();
        let surface = window.surface();
        self.roundtrip(id);

        // Commit a buffer
        let window = self.client(id).window_for_surface(&surface);
        window.attach_buffer();
        let current_serial = window.current_serial();
        assert!(current_serial.is_some());
        window.ack_and_commit();
        assert!(window.current_serial().is_none());
        self.roundtrip(id);

        let old_trees = self.pinnacle().layout_state.layout_trees.clone();

        // Let Pinnacle do a layout
        self.dispatch_until(|fixture| fixture.pinnacle().layout_state.layout_trees != old_trees);
        self.roundtrip(id);

        // Commit the layout
        let window = self.client(id).window_for_surface(&surface);
        window.ack_and_commit();
        self.roundtrip(id);

        surface
    }

    pub fn spawn_floating_window_with<F>(
        &mut self,
        id: ClientId,
        size: (i32, i32),
        mut pre_initial_commit: F,
    ) -> WlSurface
    where
        F: FnMut(&mut Window),
    {
        // Add a window
        let window = self.client(id).create_window();
        window.set_min_size(size.0, size.1);
        window.set_max_size(size.0, size.1);
        pre_initial_commit(window);
        window.commit();
        let surface = window.surface();
        self.roundtrip(id);

        // Commit a buffer
        let window = self.client(id).window_for_surface(&surface);
        window.attach_buffer();
        window.set_size(size.0, size.1);
        window.ack_and_commit();
        self.roundtrip(id);

        surface
    }

    pub fn spawn_windows(&mut self, amount: u8, id: ClientId) -> Vec<WlSurface> {
        let surfaces = (0..amount)
            .map(|_| self.spawn_window_with(id, |_| ()))
            .collect::<Vec<_>>();

        for surf in surfaces.iter() {
            let window = self.client(id).window_for_surface(surf);
            window.ack_and_commit();
        }

        self.roundtrip(id);

        surfaces
    }

    pub fn client(&mut self, id: ClientId) -> &mut Client {
        self.state.client(id)
    }
}

impl State {
    pub fn client(&mut self, id: ClientId) -> &mut Client {
        self.clients
            .iter_mut()
            .find(|client| client.id() == id)
            .unwrap()
    }
}

#[macro_export]
macro_rules! spawn_lua_blocking {
    ($fixture:expr, $($code:tt)*) => {{
        let join = ::std::thread::spawn(move || {
            let lua = $crate::common::new_lua();
            let task = lua.load(::mlua::chunk! {
                Pinnacle.run(function()
                    local run = function()
                        $($code)*
                    end

                    local success, err = pcall(run)

                    if not success then
                        error(err)
                    end
                end)
            });

            if let Err(err) = task.exec() {
                panic!("lua panicked: {err}");
            }
        });

        $fixture.dispatch_until(|_| join.is_finished());
        join.join().unwrap();
    }};
}
