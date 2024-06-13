// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![deny(elided_lifetimes_in_paths)]
#![warn(missing_docs)]

//! The Rust implementation of [Pinnacle](https://github.com/pinnacle-comp/pinnacle)'s
//! configuration API.
//!
//! This library allows you to interface with the Pinnacle compositor and configure various aspects
//! like input and the tag system.
//!
//! # Configuration
//!
//! ## 1. Create a cargo project
//! To create your own Rust config, create a Cargo project in `~/.config/pinnacle`.
//!
//! ## 2. Create `metaconfig.toml`
//! Then, create a file named `metaconfig.toml`. This is the file Pinnacle will use to determine
//! what to run, kill and reload-config keybinds, an optional socket directory, and any environment
//! variables to give the config client.
//!
//! In `metaconfig.toml`, put the following:
//! ```toml
//! # `command` will tell Pinnacle to run `cargo run` in your config directory.
//! # You can add stuff like "--release" here if you want to.
//! command = ["cargo", "run"]
//!
//! # You must define a keybind to reload your config if it crashes, otherwise you'll get stuck if
//! # the Lua config doesn't kick in properly.
//! reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
//!
//! # Similarly, you must define a keybind to kill Pinnacle.
//! kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }
//!
//! # You can specify an optional socket directory if you need to place the socket Pinnacle will
//! # use for configuration in a different place.
//! # socket_dir = "your/dir/here"
//!
//! # If you need to set any environment variables for the config process, you can do so here if
//! # you don't want to do it in the config itself.
//! [envs]
//! # key = "value"
//! ```
//!
//! ## 3. Set up dependencies
//! In your `Cargo.toml`, add a dependency to `pinnacle-api`:
//!
//! ```toml
//! # Cargo.toml
//!
//! [dependencies]
//! pinnacle-api = { git = "https://github.com/pinnacle-comp/pinnacle" }
//! ```
//!
//! ## 4. Set up the main function
//! In `main.rs`, change `fn main()` to `async fn main()` and annotate it with the
//! [`pinnacle_api::config`][`crate::config`] macro. Pass in the identifier you want to bind the
//! config modules to:
//!
//! ```
//! use pinnacle_api::ApiModules;
//!
//! #[pinnacle_api::config(modules)]
//! async fn main() {
//!     // `modules` is now available in the function body.
//!     // You can deconstruct `ApiModules` to get all the config structs.
//!     let ApiModules {
//!         ..
//!     } = modules;
//! }
//! ```
//!
//! ## 5. Begin crafting your config!
//! You can peruse the documentation for things to configure.

use std::sync::Arc;

use futures::{future::BoxFuture, Future, FutureExt, StreamExt};
use input::Input;
use layout::Layout;
use output::Output;
use pinnacle::Pinnacle;
use process::Process;
use render::Render;
use signal::SignalState;
use tag::Tag;
use tokio::{
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver},
        RwLock,
    },
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use window::Window;

#[cfg(feature = "snowcap")]
use snowcap_api::layer::Layer;

pub mod input;
pub mod layout;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod render;
pub mod signal;
pub mod tag;
pub mod util;
pub mod window;

pub use pinnacle_api_macros::config;
pub use tokio;
pub use xkbcommon;

/// A struct containing static references to all of the configuration structs.
#[non_exhaustive]
#[derive(Clone)]
pub struct ApiModules {
    /// The [`Pinnacle`] struct
    pub pinnacle: &'static Pinnacle,
    /// The [`Process`] struct
    pub process: &'static Process,
    /// The [`Window`] struct
    pub window: &'static Window,
    /// The [`Input`] struct
    pub input: &'static Input,
    /// The [`Output`] struct
    pub output: &'static Output,
    /// The [`Tag`] struct
    pub tag: &'static Tag,
    /// The [`Layout`] struct
    pub layout: &'static Layout,
    /// The [`Render`] struct
    pub render: &'static Render,
    signal: Arc<RwLock<SignalState>>,

    #[cfg(feature = "snowcap")]
    /// The snowcap widget system.
    pub snowcap: &'static Layer,
}

impl std::fmt::Debug for ApiModules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiModules")
            .field("pinnacle", &self.pinnacle)
            .field("process", &self.process)
            .field("window", &self.window)
            .field("input", &self.input)
            .field("output", &self.output)
            .field("tag", &self.tag)
            .field("layout", &self.layout)
            .field("render", &self.render)
            .field("signal", &"...")
            // TODO: snowcap
            .finish()
    }
}

/// Api receivers.
pub struct Receivers {
    pinnacle: UnboundedReceiver<BoxFuture<'static, ()>>,
    #[cfg(feature = "snowcap")]
    snowcap: UnboundedReceiver<JoinHandle<()>>,
}

/// Connects to Pinnacle and builds the configuration structs.
///
/// This function is inserted at the top of your config through the [`config`] macro.
/// You should use that macro instead of this function directly.
pub async fn connect() -> Result<(ApiModules, Receivers), Box<dyn std::error::Error>> {
    // port doesn't matter, we use a unix socket
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            tokio::net::UnixStream::connect(
                std::env::var("PINNACLE_GRPC_SOCKET")
                    .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?"),
            )
        }))
        .await
        .unwrap();

    let (fut_sender, fut_recv) = unbounded_channel::<BoxFuture<'static, ()>>();

    let signal = Arc::new(RwLock::new(SignalState::new(
        channel.clone(),
        fut_sender.clone(),
    )));

    let pinnacle = Box::leak(Box::new(Pinnacle::new(channel.clone())));
    let process = Box::leak(Box::new(Process::new(channel.clone(), fut_sender.clone())));
    let window = Box::leak(Box::new(Window::new(channel.clone())));
    let input = Box::leak(Box::new(Input::new(channel.clone(), fut_sender.clone())));
    let output = Box::leak(Box::new(Output::new(channel.clone())));
    let tag = Box::leak(Box::new(Tag::new(channel.clone())));
    let render = Box::leak(Box::new(Render::new(channel.clone())));
    let layout = Box::leak(Box::new(Layout::new(channel.clone(), fut_sender.clone())));

    #[cfg(not(feature = "snowcap"))]
    let modules = ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
        layout,
        render,
        signal: signal.clone(),
    };

    #[cfg(feature = "snowcap")]
    let (snowcap, snowcap_recv) = snowcap_api::connect().await.unwrap();

    #[cfg(feature = "snowcap")]
    let modules = ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
        layout,
        render,
        signal: signal.clone(),
        snowcap: Box::leak(Box::new(snowcap)),
    };

    window.finish_init(modules.clone());
    output.finish_init(modules.clone());
    tag.finish_init(modules.clone());
    layout.finish_init(modules.clone());
    signal.read().await.finish_init(modules.clone());

    #[cfg(feature = "snowcap")]
    let receivers = Receivers {
        pinnacle: fut_recv,
        snowcap: snowcap_recv,
    };

    #[cfg(not(feature = "snowcap"))]
    let receivers = Receivers { pinnacle: fut_recv };

    Ok((modules, receivers))
}

/// Listen to Pinnacle for incoming messages.
///
/// This will run all futures returned by configuration methods that take in callbacks in order to
/// call them.
///
/// This function is inserted at the end of your config through the [`config`] macro.
/// You should use the macro instead of this function directly.
pub async fn listen(api: ApiModules, receivers: Receivers) {
    #[cfg(feature = "snowcap")]
    let Receivers {
        pinnacle: fut_recv,
        snowcap: snowcap_recv,
    } = receivers;
    #[cfg(not(feature = "snowcap"))]
    let Receivers { pinnacle: fut_recv } = receivers;

    let mut fut_recv = UnboundedReceiverStream::new(fut_recv);
    let mut set = futures::stream::FuturesUnordered::new();

    let mut shutdown_stream = api.pinnacle.shutdown_watch().await;

    let mut shutdown_watcher = async move {
        // This will trigger either when the compositor sends the shutdown signal
        // or when it exits (in which case the stream received an error)
        shutdown_stream.next().await;
    }
    .boxed();

    #[cfg(feature = "snowcap")]
    tokio::spawn(snowcap_api::listen(snowcap_recv));

    loop {
        tokio::select! {
            fut = fut_recv.next() => {
                if let Some(fut) = fut {
                    set.push(tokio::spawn(fut));
                } else {
                    break;
                }
            }
            res = set.next() => {
                if let Some(Err(join_err)) = res {
                    eprintln!("tokio task panicked: {join_err}");
                    api.signal.write().await.shutdown();
                    break;
                }
            }
            _ = &mut shutdown_watcher => {
                api.signal.write().await.shutdown();
                break;
            }
        }
    }
}

/// Block on a future using the current Tokio runtime.
pub(crate) fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(future)
    })
}
