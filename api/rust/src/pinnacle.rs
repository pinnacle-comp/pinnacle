// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Compositor management.
//!
//! This module provides general compositor actions like quitting and reloading the config.

use pinnacle_api_defs::pinnacle::{
    self,
    v1::{
        BackendRequest, KeepaliveRequest, KeepaliveResponse, QuitRequest, ReloadConfigRequest,
        SetXwaylandClientSelfScaleRequest,
    },
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

/// Sets whether or not xwayland clients should scale themselves.
///
/// If `true`, xwayland clients will be told they are on an output with a larger or smaller size than
/// normal then rescaled to replicate being on an output with a scale of 1.
///
/// Xwayland clients that support DPI scaling will scale properly, leading to crisp and correct scaling
/// with fractional output scales. Those that don't, like `xterm`, will render as if they are on outputs
/// with scale 1, and their scale will be slightly incorrect on outputs with fractional scale.
///
/// Results may vary if you have multiple outputs with different scales.
pub fn set_xwayland_self_scaling(should_self_scale: bool) {
    Client::pinnacle()
        .set_xwayland_client_self_scale(SetXwaylandClientSelfScaleRequest {
            self_scale: should_self_scale,
        })
        .block_on_tokio()
        .unwrap();
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
