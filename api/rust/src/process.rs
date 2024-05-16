// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Process management.
//!
//! This module provides [`Process`], which allows you to spawn processes and set environment
//! variables.

use futures::{future::BoxFuture, FutureExt, StreamExt};
use pinnacle_api_defs::pinnacle::process::v0alpha1::{
    process_service_client::ProcessServiceClient, SetEnvRequest, SpawnRequest,
};
use tokio::sync::mpsc::UnboundedSender;
use tonic::transport::Channel;

use crate::block_on_tokio;

/// A struct containing methods to spawn processes with optional callbacks and set environment
/// variables.
#[derive(Debug, Clone)]
pub struct Process {
    channel: Channel,
    fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
}

/// Optional callbacks to be run when a spawned process prints to stdout or stderr or exits.
#[derive(Default)]
pub struct SpawnCallbacks {
    /// A callback that will be run when a process prints to stdout with a line
    pub stdout: Option<Box<dyn FnMut(String) + Send>>,
    /// A callback that will be run when a process prints to stderr with a line
    pub stderr: Option<Box<dyn FnMut(String) + Send>>,
    /// A callback that will be run when a process exits with a status code and message
    #[allow(clippy::type_complexity)]
    pub exit: Option<Box<dyn FnMut(Option<i32>, String) + Send>>,
}

impl Process {
    pub(crate) fn new(
        channel: Channel,
        fut_sender: UnboundedSender<BoxFuture<'static, ()>>,
    ) -> Process {
        Self {
            channel,
            fut_sender,
        }
    }

    fn create_process_client(&self) -> ProcessServiceClient<Channel> {
        ProcessServiceClient::new(self.channel.clone())
    }

    /// Spawn a process.
    ///
    /// Note that windows spawned *before* tags are added will not be displayed.
    /// This will be changed in the future to be more like Awesome, where windows with no tags are
    /// displayed on every tag instead.
    ///
    /// # Examples
    ///
    /// ```
    /// process.spawn(["alacritty"]);
    /// process.spawn(["bash", "-c", "swaybg -i /path/to/wallpaper"]);
    /// ```
    pub fn spawn(&self, args: impl IntoIterator<Item = impl Into<String>>) {
        self.spawn_inner(args, false, None);
    }

    /// Spawn a process with callbacks for its stdout, stderr, and exit information.
    ///
    /// See [`SpawnCallbacks`] for the passed in struct.
    ///
    /// Note that windows spawned *before* tags are added will not be displayed.
    /// This will be changed in the future to be more like Awesome, where windows with no tags are
    /// displayed on every tag instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use pinnacle_api::process::SpawnCallbacks;
    ///
    /// process.spawn_with_callbacks(["alacritty"], SpawnCallbacks {
    ///     stdout: Some(Box::new(|line| println!("stdout: {line}"))),
    ///     stderr: Some(Box::new(|line| println!("stderr: {line}"))),
    ///     exit: Some(Box::new(|code, msg| println!("exit code: {code:?}, exit_msg: {msg}"))),
    /// });
    /// ```
    pub fn spawn_with_callbacks(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        callbacks: SpawnCallbacks,
    ) {
        self.spawn_inner(args, false, Some(callbacks));
    }

    /// Spawn a process only if it isn't already running.
    ///
    /// This is useful for startup programs.
    ///
    /// See [`Process::spawn`] for details.
    pub fn spawn_once(&self, args: impl IntoIterator<Item = impl Into<String>>) {
        self.spawn_inner(args, true, None);
    }

    /// Spawn a process only if it isn't already running with optional callbacks for its stdout,
    /// stderr, and exit information.
    ///
    /// This is useful for startup programs.
    ///
    /// See [`Process::spawn_with_callbacks`] for details.
    pub fn spawn_once_with_callbacks(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        callbacks: SpawnCallbacks,
    ) {
        self.spawn_inner(args, true, Some(callbacks));
    }

    fn spawn_inner(
        &self,
        args: impl IntoIterator<Item = impl Into<String>>,
        once: bool,
        callbacks: Option<SpawnCallbacks>,
    ) {
        let mut client = self.create_process_client();

        let args = args.into_iter().map(Into::into).collect::<Vec<_>>();

        let request = SpawnRequest {
            args,
            once: Some(once),
            has_callback: Some(callbacks.is_some()),
        };

        let mut stream = block_on_tokio(client.spawn(request)).unwrap().into_inner();

        self.fut_sender
            .send(
                async move {
                    let Some(mut callbacks) = callbacks else { return };
                    while let Some(Ok(response)) = stream.next().await {
                        if let Some(line) = response.stdout {
                            if let Some(stdout) = callbacks.stdout.as_mut() {
                                stdout(line);
                            }
                        }
                        if let Some(line) = response.stderr {
                            if let Some(stderr) = callbacks.stderr.as_mut() {
                                stderr(line);
                            }
                        }
                        if let Some(exit_msg) = response.exit_message {
                            if let Some(exit) = callbacks.exit.as_mut() {
                                exit(response.exit_code, exit_msg);
                            }
                        }
                        tokio::task::yield_now().await;
                    }
                }
                .boxed(),
            )
            .unwrap();
    }

    /// Set an environment variable for the compositor.
    /// This will cause any future spawned processes to have this environment variable.
    ///
    /// # Examples
    ///
    /// ```
    /// process.set_env("ENV", "a value lalala");
    /// ```
    pub fn set_env(&self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();

        let mut client = self.create_process_client();

        block_on_tokio(client.set_env(SetEnvRequest {
            key: Some(key),
            value: Some(value),
        }))
        .unwrap();
    }
}
