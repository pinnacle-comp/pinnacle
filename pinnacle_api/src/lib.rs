use std::{
    error::Error,
    io,
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
};

use smithay::reexports::calloop::{
    self, generic::Generic, EventSource, Interest, Mode, PostAction,
};

use crate::message::Msg;

pub mod message;

const SOCKET_PATH: &str = "/tmp/pinnacle_socket";

pub fn run() -> Result<(), Box<dyn Error>> {
    let socket_path = Path::new(SOCKET_PATH);

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    std::thread::spawn(|| handle_client(stream));
                }
                Err(err) => {
                    eprintln!("Incoming stream error: {}", err);
                }
            }
        }
    });

    Ok(())
}

fn handle_client(stream: UnixStream) {
    loop {
        let msg: Msg = rmp_serde::from_read(&stream).unwrap();

        println!("{:?}", msg);
    }
}

pub struct PinnacleSocketSource {
    socket: Generic<UnixListener>,
}

impl PinnacleSocketSource {
    pub fn new() -> Result<Self, io::Error> {
        let socket_path = Path::new(SOCKET_PATH);

        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;

        let socket = Generic::new(listener, Interest::READ, Mode::Level);

        Ok(Self { socket })
    }
}

impl EventSource for PinnacleSocketSource {
    type Event = UnixStream;

    type Metadata = ();

    type Ret = ();

    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> Result<calloop::PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.socket
            .process_events(readiness, token, |_readiness, listener| {
                listener.set_nonblocking(true)?;
                while let Ok((stream, _sock_addr)) = listener.accept() {
                    stream.set_nonblocking(true)?;
                    callback(stream, &mut ());
                }

                Ok(PostAction::Continue)
            })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.socket.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.socket.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.socket.unregister(poll)
    }
}

pub struct PinnacleStreamSource {
    stream: Generic<UnixStream>,
}

impl PinnacleStreamSource {
    pub fn new(stream: UnixStream) -> Self {
        Self {
            stream: Generic::new(stream, Interest::READ, Mode::Level),
        }
    }
}

impl EventSource for PinnacleStreamSource {
    type Event = Msg;

    type Metadata = ();

    type Ret = ();

    type Error = io::Error;

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.stream
            .process_events(readiness, token, |_readiness, stream| {
                match rmp_serde::from_read(stream as &UnixStream) {
                    Ok(msg) => callback(msg, &mut ()),
                    Err(rmp_serde::decode::Error::InvalidMarkerRead(err))
                        if err.kind() == io::ErrorKind::UnexpectedEof =>
                    {
                        stream.shutdown(std::net::Shutdown::Both)?;
                        println!("Stream closed: {:?}", err);
                        return Ok(PostAction::Remove);
                    }
                    Err(err) => println!("{:?}", err),
                }

                Ok(PostAction::Continue)
            })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.stream.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut calloop::TokenFactory,
    ) -> calloop::Result<()> {
        self.stream.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.stream.unregister(poll)
    }
}
