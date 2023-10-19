use crate::{
    msg::{Args, CallbackId, Msg},
    send_msg, CALLBACK_VEC,
};

pub struct Process;

impl Process {
    pub fn spawn(&self, command: Vec<&str>) -> anyhow::Result<()> {
        let msg = Msg::Spawn {
            command: command.into_iter().map(|s| s.to_string()).collect(),
            callback_id: None,
        };

        send_msg(msg)
    }

    pub fn spawn_with_callback<F>(&self, command: Vec<&str>, mut callback: F) -> anyhow::Result<()>
    where
        F: FnMut(Option<String>, Option<String>, Option<i32>, Option<String>) + Send + 'static,
    {
        let args_callback = move |args: Args| {
            if let Args::Spawn {
                stdout,
                stderr,
                exit_code,
                exit_msg,
            } = args
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
}
