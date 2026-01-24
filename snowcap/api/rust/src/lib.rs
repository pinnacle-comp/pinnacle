//! Snowcap: A very, *very* WIP widget system built for [Pinnacle](https://github.com/pinnacle-comp/pinnacle).
//!
//! [AwesomeWM](https://awesomewm.org/) has a widget system, and Pinnacle is heavily inspired by
//! it, thus Snowcap was created.
//!
//! Snowcap used [Iced](https://iced.rs/) along with Smithay's [client toolkit](https://github.com/Smithay/client-toolkit)
//! to draw widgets on screen. The current, *very* early API is mostly a wrapper around Iced's
//! widget API and as such closely mirrors it.
//!
//! Once Snowcap matures a bit, you'll be able to use it in other compositors as well! Many parts
//! of Snowcap are designed to be compositor-agnostic. You'll just need a compositor that
//! implements the `wlr-layer-shell` protocol.

mod client;
pub mod decoration;
pub mod input;
pub mod layer;
pub mod popup;
pub mod signal;
pub mod widget;

use client::Client;
use hyper_util::rt::TokioIo;
pub use xkbcommon;

use std::{path::PathBuf, time::Duration};

use futures::Future;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

fn socket_dir() -> PathBuf {
    xdg::BaseDirectories::with_prefix("snowcap")
        .get_runtime_directory()
        .cloned()
        .unwrap_or(PathBuf::from("/tmp"))
}

fn socket_name() -> String {
    let wayland_suffix = std::env::var("WAYLAND_DISPLAY").unwrap_or("wayland-0".into());
    format!("snowcap-grpc-{wayland_suffix}.sock")
}

/// Connect to a running Snowcap instance.
///
/// Only one snowcap instance can be open per Wayland session.
/// This function will search for a Snowcap socket at
/// `$XDG_RUNTIME_DIR/snowcap-grpc-$WAYLAND_DISPLAY.sock` and connect to it.
pub async fn connect() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| async {
            Ok::<_, std::io::Error>(TokioIo::new(
                tokio::net::UnixStream::connect(socket_dir().join(socket_name())).await?,
            ))
        }))
        .await?;

    Client::init(channel);

    Ok(())
}

/// Listen to Snowcap for events.
pub async fn listen() {
    loop {
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await
    }
}

trait BlockOnTokio {
    type Output;

    fn block_on_tokio(self) -> Self::Output;
}

impl<F: Future> BlockOnTokio for F {
    type Output = F::Output;

    /// Blocks on a future using the current Tokio runtime.
    fn block_on_tokio(self) -> Self::Output {
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(self)
        })
    }
}
