pub mod msg;

use std::{
    io::{self, Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
};

use smithay::reexports::calloop::{
    self, channel::Sender, generic::Generic, EventSource, Interest, Mode, PostAction,
};

use self::msg::{Msg, OutgoingMsg};

const SOCKET_PATH: &str = "/tmp/pinnacle_socket";

fn handle_client(mut stream: UnixStream, sender: Sender<Msg>) {
    loop {
        let mut len_marker_bytes = [0u8; 4];
        if let Err(err) = stream.read_exact(&mut len_marker_bytes) {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                tracing::warn!("stream closed: {}", err);
                stream.shutdown(std::net::Shutdown::Both).unwrap();
                break;
            }
        };

        let len_marker = u32::from_ne_bytes(len_marker_bytes);
        let mut msg_bytes = vec![0u8; len_marker as usize];

        if let Err(err) = stream.read_exact(msg_bytes.as_mut_slice()) {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                tracing::warn!("stream closed: {}", err);
                stream.shutdown(std::net::Shutdown::Both).unwrap();
                break;
            }
        };
        let msg: Msg = rmp_serde::from_slice(msg_bytes.as_slice()).unwrap(); // TODO: handle error

        sender.send(msg).unwrap();
    }
    tracing::info!("end of handle_client");
}

pub struct PinnacleSocketSource {
    socket: Generic<UnixListener>,
    sender: Sender<Msg>,
}

impl PinnacleSocketSource {
    pub fn new(sender: Sender<Msg>) -> Result<Self, io::Error> {
        let socket_path = Path::new(SOCKET_PATH);

        if let Ok(exists) = socket_path.try_exists() {
            if exists {
                std::fs::remove_file(socket_path)?;
            }
        }

        let listener = UnixListener::bind(socket_path)?;
        listener.set_nonblocking(true)?;

        let socket = Generic::new(listener, Interest::READ, Mode::Level);

        Ok(Self { socket, sender })
    }
}

pub fn send_to_client(
    stream: &mut UnixStream,
    msg: &OutgoingMsg,
) -> Result<(), rmp_serde::encode::Error> {
    let msg = rmp_serde::to_vec_named(msg)?;
    let msg_len = msg.len() as u32;
    let bytes = msg_len.to_ne_bytes();

    if let Err(err) = stream.write_all(&bytes) {
        if err.kind() == io::ErrorKind::BrokenPipe {
            // TODO: notify user that config daemon is ded
            return Ok(()); // TODO:
        }
    }
    if let Err(err) = stream.write_all(msg.as_slice()) {
        if err.kind() == io::ErrorKind::BrokenPipe {
            // TODO: something
            return Ok(()); // TODO:
        }
    };
    Ok(())
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
                while let Ok((stream, _sock_addr)) = listener.accept() {
                    let sender = self.sender.clone();
                    let callback_stream = stream.try_clone().unwrap(); // TODO: error
                    callback(callback_stream, &mut ());
                    std::thread::spawn(move || {
                        handle_client(stream, sender);
                    });
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
