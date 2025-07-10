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

pub mod input;
pub mod layer;
pub mod widget;

use hyper_util::rt::TokioIo;
use snowcap_api_defs::snowcap::{
    input::v1::input_service_client::InputServiceClient,
    layer::v1::layer_service_client::LayerServiceClient,
};
pub use xkbcommon;

use std::{path::PathBuf, sync::RwLock, time::Duration};

use futures::Future;
use layer::Layer;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

static LAYER: RwLock<Option<LayerServiceClient<Channel>>> = RwLock::new(None);
static INPUT: RwLock<Option<InputServiceClient<Channel>>> = RwLock::new(None);

pub(crate) fn layer() -> LayerServiceClient<Channel> {
    LAYER
        .read()
        .expect("grpc connection was not initialized")
        .clone()
        .unwrap()
}
pub(crate) fn input() -> InputServiceClient<Channel> {
    INPUT
        .read()
        .expect("grpc connection was not initialized")
        .clone()
        .unwrap()
}

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
pub async fn connect() -> Result<Layer, Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| async {
            Ok::<_, std::io::Error>(TokioIo::new(
                tokio::net::UnixStream::connect(socket_dir().join(socket_name())).await?,
            ))
        }))
        .await?;

    let _ = LAYER
        .write()
        .unwrap()
        .replace(LayerServiceClient::new(channel.clone()));
    let _ = INPUT
        .write()
        .unwrap()
        .replace(InputServiceClient::new(channel.clone()));

    Ok(Layer)
}

/// Listen to Snowcap for events.
pub async fn listen() {
    loop {
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await
    }
}

pub(crate) fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(future)
    })
}
