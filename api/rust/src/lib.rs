//! The Rust implementation of API for Pinnacle, a Wayland compositor.

#![warn(missing_docs)]

pub mod input;
mod msg;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod tag;
pub mod window;

/// The xkbcommon crate, re-exported for your convenience.
pub use xkbcommon;

/// The prelude for the Pinnacle API.
///
/// This contains useful imports that you will likely need.
/// To that end, you can do `use pinnacle_api::prelude::*` to
/// prevent your config file from being cluttered with imports.
pub mod prelude {
    pub use crate::input::libinput::*;
    pub use crate::input::Modifier;
    pub use crate::input::MouseButton;
    pub use crate::input::MouseEdge;
    pub use crate::output::AlignmentHorizontal;
    pub use crate::output::AlignmentVertical;
    pub use crate::tag::Layout;
    pub use crate::window::rules::WindowRule;
    pub use crate::window::rules::WindowRuleCondition;
    pub use crate::window::FloatingOrTiled;
    pub use crate::window::FullscreenOrMaximized;
}

use std::{
    collections::{hash_map::Entry, HashMap},
    convert::Infallible,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{atomic::AtomicU32, Mutex, OnceLock},
};

use msg::{Args, CallbackId, IncomingMsg, Msg, Request, RequestResponse};

use crate::msg::RequestId;

static STREAM: OnceLock<Mutex<UnixStream>> = OnceLock::new();
#[allow(clippy::type_complexity)]
static CALLBACK_VEC: Mutex<Vec<Box<dyn FnMut(Option<Args>) + Send>>> = Mutex::new(Vec::new());
lazy_static::lazy_static! {
    static ref UNREAD_CALLBACK_MSGS: Mutex<HashMap<CallbackId, IncomingMsg>> = Mutex::new(HashMap::new());
    static ref UNREAD_REQUEST_MSGS: Mutex<HashMap<RequestId, IncomingMsg>> = Mutex::new(HashMap::new());
}

static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

fn send_msg(msg: Msg) -> anyhow::Result<()> {
    let mut msg = rmp_serde::encode::to_vec_named(&msg)?;
    let mut msg_len = (msg.len() as u32).to_ne_bytes();

    let mut stream = STREAM.get().unwrap().lock().unwrap();

    stream.write_all(msg_len.as_mut_slice())?;
    stream.write_all(msg.as_mut_slice())?;

    Ok(())
}

fn read_msg(request_id: Option<RequestId>) -> IncomingMsg {
    loop {
        if let Some(request_id) = request_id {
            if let Some(msg) = UNREAD_REQUEST_MSGS.lock().unwrap().remove(&request_id) {
                return msg;
            }
        }

        let mut stream = STREAM.get().unwrap().lock().unwrap();
        let mut msg_len_bytes = [0u8; 4];
        stream.read_exact(msg_len_bytes.as_mut_slice()).unwrap();

        let msg_len = u32::from_ne_bytes(msg_len_bytes);
        let mut msg_bytes = vec![0u8; msg_len as usize];
        stream.read_exact(msg_bytes.as_mut_slice()).unwrap();

        let incoming_msg: IncomingMsg = rmp_serde::from_slice(msg_bytes.as_slice()).unwrap();

        if let Some(request_id) = request_id {
            match &incoming_msg {
                IncomingMsg::CallCallback {
                    callback_id,
                    args: _,
                } => {
                    UNREAD_CALLBACK_MSGS
                        .lock()
                        .unwrap()
                        .insert(*callback_id, incoming_msg);
                }
                IncomingMsg::RequestResponse {
                    request_id: req_id,
                    response: _,
                } => {
                    if req_id != &request_id {
                        UNREAD_REQUEST_MSGS
                            .lock()
                            .unwrap()
                            .insert(*req_id, incoming_msg);
                    } else {
                        return incoming_msg;
                    }
                }
            }
        } else {
            return incoming_msg;
        }
    }
}

fn request(request: Request) -> RequestResponse {
    use std::sync::atomic::Ordering;
    let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    let msg = Msg::Request {
        request_id: RequestId(request_id),
        request,
    };
    send_msg(msg).unwrap(); // TODO: propogate

    let IncomingMsg::RequestResponse {
        request_id: _,
        response,
    } = read_msg(Some(RequestId(request_id)))
    else {
        unreachable!()
    };

    response
}

/// Connect to Pinnacle. This needs to be called before you begin calling config functions.
///
/// This will open up a connection to the Unix socket at `$PINNACLE_SOCKET`,
/// which should be set when you start the compositor.
pub fn connect() -> anyhow::Result<()> {
    STREAM
        .set(Mutex::new(
            UnixStream::connect(PathBuf::from(
                std::env::var("PINNACLE_SOCKET").unwrap_or("/tmp/pinnacle_socket".to_string()),
            ))
            .unwrap(),
        ))
        .unwrap();

    Ok(())
}

/// Begin listening for messages coming from Pinnacle.
///
/// This needs to be called at the very end of your `setup` function.
pub fn listen() -> Infallible {
    loop {
        let mut unread_callback_msgs = UNREAD_CALLBACK_MSGS.lock().unwrap();

        for cb_id in unread_callback_msgs.keys().copied().collect::<Vec<_>>() {
            let Entry::Occupied(entry) = unread_callback_msgs.entry(cb_id) else {
                unreachable!();
            };
            let IncomingMsg::CallCallback { callback_id, args } = entry.remove() else {
                unreachable!();
            };
            let mut callback_vec = CALLBACK_VEC.lock().unwrap();
            let Some(callback) = callback_vec.get_mut(callback_id.0 as usize) else {
                unreachable!();
            };
            callback(args);
        }

        let incoming_msg = read_msg(None);

        let IncomingMsg::CallCallback { callback_id, args } = incoming_msg else {
            unreachable!();
        };

        let mut callback_vec = CALLBACK_VEC.lock().unwrap();
        let Some(callback) = callback_vec.get_mut(callback_id.0 as usize) else {
            unreachable!();
        };

        callback(args);
    }
}
