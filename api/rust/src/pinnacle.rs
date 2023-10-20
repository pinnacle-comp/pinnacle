//! Functions for compositor control, like `setup` and `quit`.

use std::{
    collections::hash_map::Entry, convert::Infallible, os::unix::net::UnixStream, path::PathBuf,
    sync::Mutex,
};

use crate::{
    msg::{IncomingMsg, Msg},
    read_msg, send_msg, CALLBACK_VEC, STREAM, UNREAD_CALLBACK_MSGS,
};

/// Quit Pinnacle.
pub fn quit() {
    send_msg(Msg::Quit).unwrap();
}

/// Setup Pinnacle.
///
/// This will attempt to connect to the socket at `$PINNACLE_SOCKET`, which should be set by the
/// compositor when opened.
///
/// It will then run your `config_func`.
///
/// Lastly, it will enter a loop to listen to messages coming from Pinnacle.
///
/// If this function returns, an error has occurred.
pub fn setup(config_func: impl FnOnce()) -> anyhow::Result<Infallible> {
    STREAM
        .set(Mutex::new(UnixStream::connect(PathBuf::from(
            std::env::var("PINNACLE_SOCKET").unwrap_or("/tmp/pinnacle_socket".to_string()),
        ))?))
        .unwrap();

    config_func();

    loop {
        let mut unread_callback_msgs = UNREAD_CALLBACK_MSGS.lock().unwrap();
        let mut callback_vec = CALLBACK_VEC.lock().unwrap();

        for cb_id in unread_callback_msgs.keys().copied().collect::<Vec<_>>() {
            let Entry::Occupied(entry) = unread_callback_msgs.entry(cb_id) else {
                unreachable!();
            };
            let IncomingMsg::CallCallback { callback_id, args } = entry.remove() else {
                unreachable!();
            };
            let Some(callback) = callback_vec.get_mut(callback_id.0 as usize) else {
                unreachable!();
            };
            callback(args);
        }

        let incoming_msg = read_msg(None);

        let IncomingMsg::CallCallback { callback_id, args } = incoming_msg else {
            unreachable!();
        };

        let Some(callback) = callback_vec.get_mut(callback_id.0 as usize) else {
            unreachable!();
        };

        callback(args);
    }
}
