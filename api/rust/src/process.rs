// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Process management.
//!
//! This module provides ways to spawn processes and handle their output.

use std::{
    collections::HashMap,
    os::fd::{FromRawFd, OwnedFd},
};

use passfd::FdPassingExt;
use pinnacle_api_defs::pinnacle::process::v1::{SpawnRequest, WaitOnSpawnRequest};
use tokio_stream::StreamExt;

use crate::{client::Client, BlockOnTokio};

/// A process builder that allows you to spawn programs.
pub struct Command {
    cmd: Vec<String>,
    envs: HashMap<String, String>,
    shell_cmd: Vec<String>,
    unique: bool,
    once: bool,
}

/// The result of spawning a [`Command`].
#[derive(Debug)]
pub struct Child {
    pid: u32,
    /// This process's standard in.
    pub stdin: Option<tokio::process::ChildStdin>,
    /// This process's standard out.
    pub stdout: Option<tokio::process::ChildStdout>,
    /// This process's standard error.
    pub stderr: Option<tokio::process::ChildStderr>,
}

/// Information from an exited process.
#[derive(Debug, Default)]
pub struct ExitInfo {
    /// The process's exit code.
    pub exit_code: Option<i32>,
    /// The process's exit message.
    pub exit_msg: Option<String>,
}

impl Child {
    /// Waits for this process to exit, blocking the current thread.
    pub fn wait(self) -> ExitInfo {
        self.wait_async().block_on_tokio()
    }

    /// Async impl for [`Self::wait`].
    pub async fn wait_async(self) -> ExitInfo {
        let mut exit_status = Client::process()
            .wait_on_spawn(WaitOnSpawnRequest { pid: self.pid })
            .await
            .unwrap()
            .into_inner();

        let thing = exit_status.next().await;

        let Some(Ok(response)) = thing else {
            return Default::default();
        };

        ExitInfo {
            exit_code: response.exit_code,
            exit_msg: response.exit_msg,
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let pid = self.pid;

        // Wait on the process so it doesn't go zombie
        tokio::spawn(async move {
            Client::process()
                .wait_on_spawn(WaitOnSpawnRequest { pid })
                .await
                .unwrap();
        });
    }
}

impl Command {
    /// Creates a new [`Command`] that will spawn the provided `program`.
    pub fn new(program: impl ToString) -> Self {
        Self {
            cmd: vec![program.to_string()],
            envs: Default::default(),
            shell_cmd: Vec::new(),
            unique: false,
            once: false,
        }
    }

    /// Adds an argument to the command.
    pub fn arg(&mut self, arg: impl ToString) -> &mut Self {
        self.cmd.push(arg.to_string());
        self
    }

    /// Adds multiple arguments to the command.
    pub fn args(&mut self, args: impl IntoIterator<Item = impl ToString>) -> &mut Self {
        self.cmd.extend(args.into_iter().map(|arg| arg.to_string()));
        self
    }

    /// Sets an environment variable that the process will spawn with.
    pub fn env(&mut self, key: impl ToString, value: impl ToString) -> &mut Self {
        self.envs.insert(key.to_string(), value.to_string());
        self
    }

    /// Sets multiple environment variables that the process will spawn with.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        self.envs.extend(
            vars.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    /// Causes this command to only spawn the program if it is the only instance currently running.
    pub fn unique(&mut self) -> &mut Self {
        self.unique = true;
        self
    }

    /// Causes this command to spawn the program exactly once in the compositor's lifespan.
    pub fn once(&mut self) -> &mut Self {
        self.once = true;
        self
    }

    /// Spawns this command, returning the spawned process's standard io, if any.
    pub fn spawn(&mut self) -> Option<Child> {
        let data = Client::process()
            .spawn(SpawnRequest {
                cmd: self.cmd.clone(),
                unique: self.unique,
                once: self.once,
                shell_cmd: self.shell_cmd.clone(),
                envs: self.envs.clone(),
            })
            .block_on_tokio()
            .unwrap()
            .into_inner()
            .spawn_data?;

        let pid = data.pid;
        let fd_socket_path = data.fd_socket_path;

        let mut stdin = None;
        let mut stdout = None;
        let mut stderr = None;

        let stream = std::os::unix::net::UnixStream::connect(fd_socket_path)
            .expect("this should be set up by the compositor");

        if data.has_stdin {
            let fd = stream.recv_fd().unwrap();
            let child_stdin =
                tokio::process::ChildStdin::from_std(std::process::ChildStdin::from(unsafe {
                    OwnedFd::from_raw_fd(fd)
                }))
                .unwrap();
            stdin = Some(child_stdin);
        }

        if data.has_stdout {
            let fd = stream.recv_fd().unwrap();
            let child_stdout =
                tokio::process::ChildStdout::from_std(std::process::ChildStdout::from(unsafe {
                    OwnedFd::from_raw_fd(fd)
                }))
                .unwrap();
            stdout = Some(child_stdout);
        }

        if data.has_stderr {
            let fd = stream.recv_fd().unwrap();
            let child_stderr =
                tokio::process::ChildStderr::from_std(std::process::ChildStderr::from(unsafe {
                    OwnedFd::from_raw_fd(fd)
                }))
                .unwrap();
            stderr = Some(child_stderr);
        }

        Some(Child {
            pid,
            stdin,
            stdout,
            stderr,
        })
    }
}
