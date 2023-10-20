mod input;
mod msg;
mod output;
mod process;
mod tag;
mod window;

use input::Input;
pub use input::MouseButton;
pub use msg::Modifier;
pub use msg::MouseEdge;
use output::Output;
use tag::Tag;
use window::Window;
pub use xkbcommon::xkb::keysyms;
pub use xkbcommon::xkb::Keysym;

use std::{
    collections::HashMap,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{atomic::AtomicU32, Mutex, OnceLock},
};

use msg::{Args, CallbackId, IncomingMsg, Msg, Request, RequestResponse};
use process::Process;

use crate::msg::RequestId;

static STREAM: OnceLock<Mutex<UnixStream>> = OnceLock::new();
#[allow(clippy::type_complexity)]
static CALLBACK_VEC: Mutex<Vec<Box<dyn FnMut(Option<Args>) + Send>>> = Mutex::new(Vec::new());
lazy_static::lazy_static! {
    static ref UNREAD_CALLBACK_MSGS: Mutex<HashMap<CallbackId, IncomingMsg>> = Mutex::new(HashMap::new());
    static ref UNREAD_REQUEST_MSGS: Mutex<HashMap<RequestId, IncomingMsg>> = Mutex::new(HashMap::new());
}

static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Setup Pinnacle.
pub fn setup(config_func: impl FnOnce(Pinnacle)) -> anyhow::Result<()> {
    STREAM
        .set(Mutex::new(UnixStream::connect(PathBuf::from(
            std::env::var("PINNACLE_SOCKET").unwrap_or("/tmp/pinnacle_socket".to_string()),
        ))?))
        .unwrap();

    let pinnacle = Pinnacle {
        process: Process,
        input: Input,
        window: Window,
        output: Output,
        tag: Tag,
    };

    config_func(pinnacle);

    loop {
        let mut unread_callback_msgs = UNREAD_CALLBACK_MSGS.lock().unwrap();
        let mut callback_vec = CALLBACK_VEC.lock().unwrap();
        let mut to_remove = vec![];
        for (cb_id, incoming_msg) in unread_callback_msgs.iter() {
            let IncomingMsg::CallCallback { callback_id, args } = incoming_msg else {
                continue;
            };
            let Some(f) = callback_vec.get_mut(callback_id.0 as usize) else {
                continue;
            };
            f(args.clone());
            to_remove.push(*cb_id);
        }
        for id in to_remove {
            unread_callback_msgs.remove(&id);
        }

        let incoming_msg = read_msg(None);

        assert!(matches!(incoming_msg, IncomingMsg::CallCallback { .. }));

        let IncomingMsg::CallCallback { callback_id, args } = incoming_msg else {
            unreachable!()
        };

        let Some(f) = callback_vec.get_mut(callback_id.0 as usize) else {
            continue;
        };

        f(args);
    }
}

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

pub struct Pinnacle {
    pub process: Process,
    pub window: Window,
    pub input: Input,
    pub output: Output,
    pub tag: Tag,
}
