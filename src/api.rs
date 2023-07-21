// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Pinnacle's configuration API.
//!
//! The API works as follows:
//!
//! - All configuration is done through a Unix socket located at /tmp/pinnacle_socket.
//! - Pinnacle is built with the intent of configuration in Lua (and possibly other languages in
//!   the future). To achieve this, an always running process in the target language needs to be
//!   spawned, known as the *client*. This allows Pinnacle, the *server*, to call into user-defined
//!   state through callback functions.
//! - The client must:
//!     - Connect to the socket,
//!     - send configuration messages through the socket, and finally
//!     - listen to requests for callbacks,
//!   in that order.
//!
//! You may be asking, "what messages am I supposed to send and receive?"
//! Great question!
//!
//! Pinnacle uses [MessagePack](https://msgpack.org/index.html) as the message format.
//! Messages should be serialized into MessagePack according to the [defined structs](msg::Msg).
//!
//! When Pinnacle needs to call a user-defined callback, for example from a keybind setting, it
//! sends a [CallCallback](msg::OutgoingMsg::CallCallback) message to the client. This message
//! contains a callback_id to identify what callback the client needs to run—but wait, where do you get that?
//!
//! The callback_id is created by the client to identify one of its callbacks. You will probably
//! need to store all callbacks in some central data structure along with a way to associate an id with it.
//! This could be an array and its indices or a hashmap and its keys (keep in mind the id needs to
//! be an unsigned 32 bit int).
//!
//! TODO: expand
//!
//! For an example, look at the Lua implementation in the repository.

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

fn handle_client(
    mut stream: UnixStream,
    sender: Sender<Msg>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let mut len_marker_bytes = [0u8; 4];
        if let Err(err) = stream.read_exact(&mut len_marker_bytes) {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                tracing::warn!("stream closed: {}", err);
                stream.shutdown(std::net::Shutdown::Both)?;
                break Ok(());
            }
        };

        let len_marker = u32::from_ne_bytes(len_marker_bytes);
        let mut msg_bytes = vec![0u8; len_marker as usize];

        if let Err(err) = stream.read_exact(msg_bytes.as_mut_slice()) {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                tracing::warn!("stream closed: {}", err);
                stream.shutdown(std::net::Shutdown::Both)?;
                break Ok(());
            }
        };
        let msg: Msg = rmp_serde::from_slice(msg_bytes.as_slice())?; // TODO: handle error

        sender.send(msg)?;
    }
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
    tracing::debug!("Sending {msg:?}");
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
                    let callback_stream = match stream.try_clone() {
                        Ok(callback_stream) => callback_stream,
                        Err(err) => return Err(err),
                    };
                    callback(callback_stream, &mut ());
                    std::thread::spawn(move || {
                        if let Err(err) = handle_client(stream, sender) {
                            tracing::error!("handle_client errored: {err}");
                        }
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
