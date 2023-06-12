use std::{
    error::Error,
    io::Read,
    mem::size_of,
    os::{
        fd::AsRawFd,
        unix::net::{UnixListener, UnixStream},
    },
    path::Path,
};

const SOCKET_PATH: &str = "/tmp/pinnacle_socket";

pub fn run() -> Result<(), Box<dyn Error>> {
    let socket_path = Path::new(SOCKET_PATH);

    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;

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

    Ok(())
}

fn handle_client(mut stream: UnixStream) {
    let mut buf = [0u8; size_of::<Message>()];
    stream.read_exact(&mut buf).unwrap();

    let thing = rkyv::check_archived_root::<Message>(&buf).unwrap();
    println!("{}", thing.number2);
}

#[repr(C)]
#[derive(rkyv::Archive)]
#[archive(check_bytes)]
pub struct Message {
    number: u32,
    number2: u8,
}
