use crate::{
    msg::{Args, CallbackId, Msg},
    send_msg, CALLBACK_VEC,
};

/// Process management.
#[derive(Clone, Copy)]
pub struct Process;

impl Process {
    /// Spawn a process.
    ///
    /// This will use Rust's (more specifically `async_process`'s) `Command` to spawn the provided
    /// arguments. If you are using any shell syntax like `~`, you may need to spawn a shell
    /// instead. If so, you may *also* need to correctly escape the input.
    pub fn spawn(&self, command: Vec<&str>) -> anyhow::Result<()> {
        let msg = Msg::Spawn {
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
    pub fn spawn_with_callback<F>(&self, command: Vec<&str>, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Option<String>, Option<String>, Option<i32>, Option<String>) + Send + 'static,
    {
        let args_callback = move |args: Option<Args>| {
            if let Some(Args::Spawn {
                stdout,
                stderr,
                exit_code,
                exit_msg,
            }) = args
            {
                callback(stdout, stderr, exit_code, exit_msg);
            }
        };

        let mut callback_vec = CALLBACK_VEC.lock().unwrap();
        let len = callback_vec.len();
        callback_vec.push(Box::new(args_callback));

        let msg = Msg::Spawn {
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
    pub fn set_env(&self, key: &str, value: &str) {
        let msg = Msg::SetEnv {
            key: key.to_string(),
            value: value.to_string(),
        };

        send_msg(msg).unwrap();
    }
}
