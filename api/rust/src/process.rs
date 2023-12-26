//! Process management.

use crate::{
    msg::{Args, CallbackId, Msg},
    send_msg, CallbackVec,
};

/// Spawn a process.
///
/// This will use Rust's (more specifically `async_process`'s) `Command` to spawn the provided
/// arguments. If you are using any shell syntax like `~`, you may need to spawn a shell
/// instead. If so, you may *also* need to correctly escape the input.
pub fn spawn(command: Vec<&str>) -> anyhow::Result<()> {
    let msg = Msg::Spawn {
        command: command.into_iter().map(|s| s.to_string()).collect(),
        callback_id: None,
    };

    send_msg(msg)
}

/// Spawn a process only if it isn't already running.
///
/// This will use Rust's (more specifically `async_process`'s) `Command` to spawn the provided
/// arguments. If you are using any shell syntax like `~`, you may need to spawn a shell
/// instead. If so, you may *also* need to correctly escape the input.
pub fn spawn_once(command: Vec<&str>) -> anyhow::Result<()> {
    let msg = Msg::SpawnOnce {
        command: command.into_iter().map(|s| s.to_string()).collect(),
        callback_id: None,
    };

    send_msg(msg)
}

/// Spawn a process with an optional callback for its stdout, stderr, and exit information.
///
/// `callback` has the following parameters:
///  - `0`: The process's stdout printed this line.
///  - `1`: The process's stderr printed this line.
///  - `2`: The process exited with this code.
///  - `3`: The process exited with this message.
///  - `4`: A `&mut `[`CallbackVec`] for use inside the closure.
///
/// You must also pass in a mutable reference to a [`CallbackVec`] in order to store your callback.
pub fn spawn_with_callback<'a, F>(
    command: Vec<&str>,
    mut callback: F,
    callback_vec: &mut CallbackVec<'a>,
) -> anyhow::Result<()>
where
    F: FnMut(Option<String>, Option<String>, Option<i32>, Option<String>, &mut CallbackVec) + 'a,
{
    let args_callback = move |args: Option<Args>, callback_vec: &mut CallbackVec<'_>| {
        if let Some(Args::Spawn {
            stdout,
            stderr,
            exit_code,
            exit_msg,
        }) = args
        {
            callback(stdout, stderr, exit_code, exit_msg, callback_vec);
        }
    };

    let len = callback_vec.callbacks.len();
    callback_vec.callbacks.push(Box::new(args_callback));

    let msg = Msg::Spawn {
        command: command.into_iter().map(|s| s.to_string()).collect(),
        callback_id: Some(CallbackId(len as u32)),
    };

    send_msg(msg)
}

// TODO: literally copy pasted from above, but will be rewritten so meh
/// Spawn a process with an optional callback for its stdout, stderr, and exit information,
/// only if it isn't already running.
///
/// `callback` has the following parameters:
///  - `0`: The process's stdout printed this line.
///  - `1`: The process's stderr printed this line.
///  - `2`: The process exited with this code.
///  - `3`: The process exited with this message.
///  - `4`: A `&mut `[`CallbackVec`] for use inside the closure.
///
/// You must also pass in a mutable reference to a [`CallbackVec`] in order to store your callback.
pub fn spawn_once_with_callback<'a, F>(
    command: Vec<&str>,
    mut callback: F,
    callback_vec: &mut CallbackVec<'a>,
) -> anyhow::Result<()>
where
    F: FnMut(Option<String>, Option<String>, Option<i32>, Option<String>, &mut CallbackVec) + 'a,
{
    let args_callback = move |args: Option<Args>, callback_vec: &mut CallbackVec<'_>| {
        if let Some(Args::Spawn {
            stdout,
            stderr,
            exit_code,
            exit_msg,
        }) = args
        {
            callback(stdout, stderr, exit_code, exit_msg, callback_vec);
        }
    };

    let len = callback_vec.callbacks.len();
    callback_vec.callbacks.push(Box::new(args_callback));

    let msg = Msg::SpawnOnce {
        command: command.into_iter().map(|s| s.to_string()).collect(),
        callback_id: Some(CallbackId(len as u32)),
    };

    send_msg(msg)
}

/// Set an environment variable for Pinnacle. All future processes spawned will have this env set.
///
/// Note that this will only set the variable for the compositor, not the running config process.
/// If you need to set an environment variable for this config, place them in the `metaconfig.toml` file instead
/// or use [`std::env::set_var`].
pub fn set_env(key: &str, value: &str) {
    let msg = Msg::SetEnv {
        key: key.to_string(),
        value: value.to_string(),
    };

    send_msg(msg).unwrap();
}
