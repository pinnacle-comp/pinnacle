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
//! In your `Cargo.toml`, add `pinnacle-api` as a dependency:
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
//! [`pinnacle_api::config`][`crate::config`] macro:
//!
//! ```
//! #[pinnacle_api::config]
//! async fn main() {}
//! ```
//!
//! ## 5. Begin crafting your config!
//!
//! You can create the API modules with [`ApiModules::new`]:
//!
//! ```
//! use pinnacle_api::ApiModules;
//!
//! let ApiModules {
//!     ..
//! } = ApiModules::new();
//! ```
//!
//! Most modules are copy-able unit structs, so you can also use them directly:
//!
//! ```
//! let _ = pinnacle_api::window::Window.get_all();
//! pinnacle_api::pinnacle::Pinnacle.quit();
//! ```
//!
//! You can peruse the documentation for things to configure.

use futures::{Future, StreamExt};
use hyper_util::rt::TokioIo;
use input::Input;
use layout::Layout;
use output::Output;
use pinnacle::Pinnacle;
use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::input_service_client::InputServiceClient,
    layout::v0alpha1::layout_service_client::LayoutServiceClient,
    output::v0alpha1::output_service_client::OutputServiceClient,
    process::v0alpha1::process_service_client::ProcessServiceClient,
    render::v0alpha1::render_service_client::RenderServiceClient,
    signal::v0alpha1::signal_service_client::SignalServiceClient,
    tag::v0alpha1::tag_service_client::TagServiceClient,
    v0alpha1::pinnacle_service_client::PinnacleServiceClient,
    window::v0alpha1::window_service_client::WindowServiceClient,
};
use process::Process;
use render::Render;
use signal::SignalState;
#[cfg(feature = "snowcap")]
use snowcap::Snowcap;
use tag::Tag;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard, RwLock};
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;
use tracing::info;
use window::Window;

pub mod input;
pub mod layout;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod render;
pub mod signal;
#[cfg(feature = "snowcap")]
pub mod snowcap;
pub mod tag;
pub mod util;
pub mod window;

pub use pinnacle_api_macros::config;
#[cfg(feature = "snowcap")]
pub use snowcap_api;
pub use tokio;
pub use xkbcommon;

// These are all `RwLock<Option>` instead of `OnceLock` purely for the fact that
// tonic doesn't like it when you use clients across tokio runtimes, and these are static
// meaning they would get reused, so this allows us to recreate the client on a
// different runtime when testing.
static PINNACLE: RwLock<Option<PinnacleServiceClient<Channel>>> = RwLock::const_new(None);
static PROCESS: RwLock<Option<ProcessServiceClient<Channel>>> = RwLock::const_new(None);
static WINDOW: RwLock<Option<WindowServiceClient<Channel>>> = RwLock::const_new(None);
static INPUT: RwLock<Option<InputServiceClient<Channel>>> = RwLock::const_new(None);
static OUTPUT: RwLock<Option<OutputServiceClient<Channel>>> = RwLock::const_new(None);
static TAG: RwLock<Option<TagServiceClient<Channel>>> = RwLock::const_new(None);
static LAYOUT: RwLock<Option<LayoutServiceClient<Channel>>> = RwLock::const_new(None);
static RENDER: RwLock<Option<RenderServiceClient<Channel>>> = RwLock::const_new(None);
static SIGNAL: RwLock<Option<SignalServiceClient<Channel>>> = RwLock::const_new(None);

static SIGNAL_MODULE: Mutex<Option<SignalState>> = Mutex::const_new(None);

pub(crate) fn pinnacle() -> PinnacleServiceClient<Channel> {
    block_on_tokio(PINNACLE.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn process() -> ProcessServiceClient<Channel> {
    block_on_tokio(PROCESS.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn window() -> WindowServiceClient<Channel> {
    block_on_tokio(WINDOW.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn input() -> InputServiceClient<Channel> {
    block_on_tokio(INPUT.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn output() -> OutputServiceClient<Channel> {
    block_on_tokio(OUTPUT.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn tag() -> TagServiceClient<Channel> {
    block_on_tokio(TAG.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn layout() -> LayoutServiceClient<Channel> {
    block_on_tokio(LAYOUT.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn render() -> RenderServiceClient<Channel> {
    block_on_tokio(RENDER.read())
        .clone()
        .expect("grpc connection was not initialized")
}
pub(crate) fn signal() -> SignalServiceClient<Channel> {
    block_on_tokio(SIGNAL.read())
        .clone()
        .expect("grpc connection was not initialized")
}

pub(crate) fn signal_module() -> MappedMutexGuard<'static, SignalState> {
    MutexGuard::map(block_on_tokio(SIGNAL_MODULE.lock()), |state| {
        state.as_mut().expect("grpc connection was not initialized")
    })
}

/// A struct containing all of the config module structs.
///
/// Everything in here is a static reference because even though the modules are
/// copy-able unit structs, you still have to put `move` when using them in closures,
/// so this is just a minor quality-of-life thing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    #[cfg(feature = "snowcap")]
    /// The snowcap widget system.
    pub snowcap: &'static Snowcap,
}

impl Default for ApiModules {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiModules {
    /// Creates all the API modules.
    pub const fn new() -> Self {
        Self {
            pinnacle: &Pinnacle,
            process: &Process,
            window: &Window,
            input: &Input,
            output: &Output,
            tag: &Tag,
            layout: &Layout,
            render: &Render,
            #[cfg(feature = "snowcap")]
            snowcap: {
                const SNOWCAP: Snowcap = Snowcap::new();
                &SNOWCAP
            },
        }
    }
}

/// Connects to Pinnacle and builds the configuration structs.
///
/// This function is inserted at the top of your config through the [`config`] macro.
/// You should use that macro instead of this function directly.
pub async fn connect() -> Result<(), Box<dyn std::error::Error>> {
    // port doesn't matter, we use a unix socket
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| async {
            let path = std::env::var("PINNACLE_GRPC_SOCKET")
                .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?");

            Ok::<_, std::io::Error>(TokioIo::new(tokio::net::UnixStream::connect(path).await?))
        }))
        .await
        .unwrap();

    let socket_path = std::env::var("PINNACLE_GRPC_SOCKET").unwrap();
    info!("Connected to {socket_path}");

    PINNACLE
        .write()
        .await
        .replace(PinnacleServiceClient::new(channel.clone()));
    PROCESS
        .write()
        .await
        .replace(ProcessServiceClient::new(channel.clone()));
    WINDOW
        .write()
        .await
        .replace(WindowServiceClient::new(channel.clone()));
    INPUT
        .write()
        .await
        .replace(InputServiceClient::new(channel.clone()));
    OUTPUT
        .write()
        .await
        .replace(OutputServiceClient::new(channel.clone()));
    TAG.write()
        .await
        .replace(TagServiceClient::new(channel.clone()));
    RENDER
        .write()
        .await
        .replace(RenderServiceClient::new(channel.clone()));
    LAYOUT
        .write()
        .await
        .replace(LayoutServiceClient::new(channel.clone()));
    SIGNAL
        .write()
        .await
        .replace(SignalServiceClient::new(channel.clone()));

    SIGNAL_MODULE.lock().await.replace(SignalState::new());

    #[cfg(feature = "snowcap")]
    snowcap_api::connect().await.unwrap();

    Ok(())
}

/// Listen to Pinnacle for incoming messages.
///
/// This will run all futures returned by configuration methods that take in callbacks in order to
/// call them.
///
/// This function is inserted at the end of your config through the [`config`] macro.
/// You should use the macro instead of this function directly.
pub async fn listen() {
    let mut shutdown_stream = Pinnacle.shutdown_watch().await;

    // This will trigger either when the compositor sends the shutdown signal
    // or when it exits (in which case the stream received an error)
    shutdown_stream.next().await;

    signal_module().shutdown();
}

/// Sets the default `tracing_subscriber` to output logs.
///
/// This subscriber does not include the time or ansi escape codes.
/// If you would like to disable this in [`crate::config`], pass in
/// `internal_tracing = false`.
pub fn set_default_tracing_subscriber() {
    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .init();
}

/// Block on a future using the current Tokio runtime.
pub(crate) fn block_on_tokio<F: Future>(future: F) -> F::Output {
    tokio::task::block_in_place(|| {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(future)
    })
}
