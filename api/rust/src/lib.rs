mod msg;
mod process;

use std::{
    io::Write,
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use msg::{Args, Msg, Request};
use process::Process;

static STREAM: OnceLock<Mutex<UnixStream>> = OnceLock::new();
#[allow(clippy::type_complexity)]
static CALLBACK_VEC: Mutex<Vec<Box<dyn FnMut(Args) + Send>>> = Mutex::new(Vec::new());

pub fn setup(config_func: impl FnOnce(Pinnacle)) -> anyhow::Result<()> {
    STREAM
        .set(Mutex::new(UnixStream::connect(PathBuf::from(
            std::env::var("PINNACLE_SOCKET").unwrap_or("/tmp/pinnacle_socket".to_string()),
        ))?))
        .unwrap();

    let pinnacle = Pinnacle { process: Process };

    config_func(pinnacle);

    Ok(())
}

fn send_msg(msg: Msg) -> anyhow::Result<()> {
    let mut msg = rmp_serde::encode::to_vec_named(&msg)?;
    let mut msg_len = (msg.len() as u32).to_ne_bytes();

    let mut stream = STREAM.get().unwrap().lock().unwrap();

    stream.write_all(msg_len.as_mut_slice())?;
    stream.write_all(msg.as_mut_slice())?;

    Ok(())
}

fn read_msg() {
    todo!()
}

fn request(request: Request) {
    //
}

pub struct Pinnacle {
    pub process: Process,
}

impl Pinnacle {}
