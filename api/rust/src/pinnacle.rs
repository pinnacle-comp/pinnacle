// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compositor management.
//!
//! This module provides general compositor actions like quitting and reloading the config.

use pinnacle_api_defs::pinnacle::{
    self,
    v1::{BackendRequest, KeepaliveRequest, KeepaliveResponse, QuitRequest, ReloadConfigRequest},
};
use tonic::Streaming;

use crate::{client::Client, BlockOnTokio};

/// A backend that Pinnacle runs with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Backend {
    /// Pinnacle is running in a tty, possibly started through a display manager.
    Tty,
    /// Pinnacle is running in a window inside another compositor, window manager,
    /// or desktop environment.
    Window,
}

/// Quits Pinnacle.
///
/// # Examples
///
/// ```
/// // Quits Pinnacle. What else were you expecting?
/// pinnacle.quit();
/// ```
pub fn quit() {
    // Ignore errors here, the config is meant to be killed
    let _ = Client::pinnacle().quit(QuitRequest {}).block_on_tokio();
}

/// Reloads the currently active config.
pub fn reload_config() {
    // Ignore errors here, the config is meant to be killed
    let _ = Client::pinnacle()
        .reload_config(ReloadConfigRequest {})
        .block_on_tokio();
}

/// Gets the currently running [`Backend`].
pub fn backend() -> Backend {
    let backend = Client::pinnacle()
        .backend(BackendRequest {})
        .block_on_tokio()
        .unwrap()
        .into_inner()
        .backend();

    match backend {
        pinnacle::v1::Backend::Unspecified => panic!("received unspecified backend"),
        pinnacle::v1::Backend::Window => Backend::Window,
        pinnacle::v1::Backend::Tty => Backend::Tty,
    }
}

pub(crate) async fn keepalive() -> (
    tokio::sync::mpsc::Sender<KeepaliveRequest>,
    Streaming<KeepaliveResponse>,
) {
    let (send, recv) = tokio::sync::mpsc::channel::<KeepaliveRequest>(5);
    let recv = tokio_stream::wrappers::ReceiverStream::new(recv);
    let streaming = Client::pinnacle()
        .keepalive(recv)
        .await
        .unwrap()
        .into_inner();
    (send, streaming)
}
