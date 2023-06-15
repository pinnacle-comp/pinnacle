use std::{
    error::Error,
    os::unix::net::{UnixListener, UnixStream},
    path::Path,
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
