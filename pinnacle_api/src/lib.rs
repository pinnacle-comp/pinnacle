use std::{
    io::Read,
    mem::size_of,
    os::unix::net::{UnixDatagram, UnixListener, UnixStream},
};

const SOCKET_PATH: &str = "/tmp/pinnacle_socket";

pub fn new_socket() {
    let socket = UnixDatagram::bind("/something/other").unwrap();
    let socket2 = UnixStream::connect("/fsalkfhgtew").unwrap();
}

pub fn start() {
    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(socket) => socket,
        Err(err) => {
            eprintln!("Failed to create UnixListener: {}", err);
            return;
        }
    };

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
}

fn handle_client(mut stream: UnixStream) {
    let mut buf = [0u8; size_of::<Message>()];
    stream.read_exact(&mut buf).unwrap();

    let thing = rkyv::check_archived_root::<Message>(&buf).unwrap();
    println!("{}", thing.msg);
}

#[repr(C)]
#[derive(rkyv::Archive)]
#[archive(check_bytes)]
struct Message {
    number: u32,
    msg: String,
    num2: u64,
    num3: u8,
}
