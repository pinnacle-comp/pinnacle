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
//!         pinnacle,
//!         process,
//!         window,
//!         input,
//!         output,
//!         tag,
//!     } = modules;
//! }
//! ```
//!
//! ## 5. Begin crafting your config!
//! You can peruse the documentation for things to configure.

use std::sync::OnceLock;

use futures::{
    channel::mpsc::UnboundedReceiver, future::BoxFuture, stream::FuturesUnordered, StreamExt,
};
use input::Input;
use output::Output;
use pinnacle::Pinnacle;
use process::Process;
use tag::Tag;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use window::Window;

pub mod input;
pub mod output;
pub mod pinnacle;
pub mod process;
pub mod tag;
pub mod util;
pub mod window;

pub use pinnacle_api_macros::config;
pub use tokio;
pub use xkbcommon;

static PINNACLE: OnceLock<Pinnacle> = OnceLock::new();
static PROCESS: OnceLock<Process> = OnceLock::new();
static WINDOW: OnceLock<Window> = OnceLock::new();
static INPUT: OnceLock<Input> = OnceLock::new();
static OUTPUT: OnceLock<Output> = OnceLock::new();
static TAG: OnceLock<Tag> = OnceLock::new();

/// A struct containing static references to all of the configuration structs.
#[derive(Debug, Clone, Copy)]
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
}

/// Connects to Pinnacle and builds the configuration structs.
///
/// This function is inserted at the top of your config through the [`config`] macro.
/// You should use that macro instead of this function directly.
pub async fn connect(
) -> Result<(ApiModules, UnboundedReceiver<BoxFuture<'static, ()>>), Box<dyn std::error::Error>> {
    let channel = Endpoint::try_from("http://[::]:50051")? // port doesn't matter, we use a unix socket
        .connect_with_connector(service_fn(|_: Uri| {
            tokio::net::UnixStream::connect(
                std::env::var("PINNACLE_GRPC_SOCKET")
                    .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?"),
            )
        }))
        .await?;

    let (fut_sender, fut_recv) = futures::channel::mpsc::unbounded::<BoxFuture<()>>();

    let output = Output::new(channel.clone(), fut_sender.clone());

    let pinnacle = PINNACLE.get_or_init(|| Pinnacle::new(channel.clone()));
    let process = PROCESS.get_or_init(|| Process::new(channel.clone(), fut_sender.clone()));
    let window = WINDOW.get_or_init(|| Window::new(channel.clone()));
    let input = INPUT.get_or_init(|| Input::new(channel.clone(), fut_sender.clone()));
    let tag = TAG.get_or_init(|| Tag::new(channel.clone(), fut_sender.clone()));
    let output = OUTPUT.get_or_init(|| output);

    let modules = ApiModules {
        pinnacle,
        process,
        window,
        input,
        output,
        tag,
    };

    Ok((modules, fut_recv))
}

/// Listen to Pinnacle for incoming messages.
///
/// This will run all futures returned by configuration methods that take in callbacks in order to
/// call them.
///
/// This function is inserted at the end of your config through the [`config`] macro.
/// You should use the macro instead of this function directly.
pub async fn listen(fut_recv: UnboundedReceiver<BoxFuture<'static, ()>>) {
    let mut future_set = FuturesUnordered::<
        BoxFuture<(
            Option<BoxFuture<()>>,
            Option<UnboundedReceiver<BoxFuture<()>>>,
        )>,
    >::new();

    future_set.push(Box::pin(async move {
        let (fut, stream) = fut_recv.into_future().await;
        (fut, Some(stream))
    }));

    while let Some((fut, stream)) = future_set.next().await {
        if let Some(fut) = fut {
            future_set.push(Box::pin(async move {
                fut.await;
                (None, None)
            }));
        }
        if let Some(stream) = stream {
            future_set.push(Box::pin(async move {
                let (fut, stream) = stream.into_future().await;
                (fut, Some(stream))
            }))
        }
    }
}
