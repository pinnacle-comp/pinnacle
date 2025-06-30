use std::{
    os::fd::AsFd,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
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

use super::{
    client::{Client, ClientId},
    server::Server,
};

pub struct Fixture {
    event_loop: EventLoop<'static, State>,
    state: State,
}

struct State {
    server: Server,
    clients: Vec<Client>,
}

static OUTPUT_COUNTER: AtomicU32 = AtomicU32::new(0);

impl Fixture {
    pub fn new() -> Self {
        let state = State {
            server: Server::new(),
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

        Self { event_loop, state }
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
    pub fn spawn_blocking<F, T>(&mut self, spawn: F) -> T
    where
        F: FnMut() -> T + Send + 'static,
        T: Send + 'static,
    {
        let handle = self.runtime_handle();
        let _guard = handle.enter();
        let join = handle.spawn_blocking(spawn);
        self.dispatch_until(|_| join.is_finished());
        self.runtime_handle().block_on(join).unwrap()
    }

    pub fn roundtrip(&mut self, id: ClientId) {
        let client = self.client(id);
        let wait = client.send_sync();
        while !wait.load(Ordering::Relaxed) {
            self.dispatch();
        }
        debug!(client = ?id, "roundtripped");
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
