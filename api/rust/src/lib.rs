// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![warn(missing_docs)]

//! The Rust implementation of [Pinnacle](https://github.com/pinnacle-comp/pinnacle)'s
//! configuration API.
//!
//! This library allows you to interface with the Pinnacle compositor and configure various aspects
//! like input and the tag system.
//!
//! # Configuration
//!
//! ## With the config generation CLI
//!
//! To create a Rust config using the config generation CLI, run
//!
//! ```sh
//! pinnacle config gen
//! ```
//!
//! and step through the interactive generator (be sure to select Rust as the language).
//! This will create the default config in the specified directory.
//!
//! ## Manually
//!
//! ### 1. Create a Cargo project
//!
//! Create a Cargo project in your config directory with `cargo init`.
//!
//! ### 2. Create `pinnacle.toml`
//!
//! `pinnacle.toml` is what tells Pinnacle what command is used to start the config.
//!
//! Create `pinnacle.toml` at the root of the cargo project and add the following to it:
//! ```toml
//! run = ["cargo", "run"]
//! ```
//!
//! Pinnacle will now use `cargo run` to start your config.
//!
//! ## 3. Set up dependencies
//!
//! In your `Cargo.toml`, add `pinnacle-api` as a dependency:
//!
//! ```toml
//! [dependencies]
//! pinnacle-api = { git = "https://github.com/pinnacle-comp/pinnacle" }
//! ```
//!
//! ## 4. Set up the main function
//!
//! In `main.rs`, remove the main function and create an `async` one. This is where your config
//! will start from. Then, call the [`main`] macro, which will create a `tokio` main function
//! that will perform the necessary setup and call your async function.
//!
//! ```
//! async fn config() {
//!     // Your config here
//! }
//!
//! pinnacle_api::main!(config);
//! ```
//!
//! ## 5. Begin crafting your config!
//!
//! Take a look at the default config or browse the docs to see what you can do.

use client::Client;
use futures::{Future, StreamExt};
use hyper_util::rt::TokioIo;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

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

mod client;

pub use tokio;
pub use xkbcommon::xkb::Keysym;

const SOCKET_PATH: &str = "PINNACLE_GRPC_SOCKET";

/// Connects to Pinnacle.
///
/// This function is called by the [`main`] and [`config`] macros.
/// You'll only need to use this if you aren't using them.
pub async fn connect() -> Result<(), Box<dyn std::error::Error>> {
    // port doesn't matter, we use a unix socket
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| async {
            let path = std::env::var(SOCKET_PATH)
                .expect("PINNACLE_GRPC_SOCKET was not set; is Pinnacle running?");

            Ok::<_, std::io::Error>(TokioIo::new(tokio::net::UnixStream::connect(path).await?))
        }))
        .await
        .unwrap();

    let socket_path = std::env::var(SOCKET_PATH).unwrap();
    println!("Connected to {socket_path}");

    Client::init(channel.clone());

    #[cfg(feature = "snowcap")]
    snowcap_api::connect().await.unwrap();

    Ok(())
}

/// Blocks until Pinnacle exits.
///
/// This function is called by the [`main`] and [`config`] macros.
/// You'll only need to use this if you aren't using them.
pub async fn block() {
    let (_sender, mut keepalive_stream) = crate::pinnacle::keepalive().await;

    // This will trigger either when the compositor sends the shutdown signal
    // or when it exits (in which case the stream receives an error)
    keepalive_stream.next().await;

    Client::signal_state().shutdown();
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

/// Defines the config's main entry point.
///
/// This macro creates a `main` function annotated with
/// `#[pinnacle_api::tokio::main]` that performs necessary setup
/// and calls the provided async function.
///
/// # Examples
///
/// ```no_run
/// async fn config() {}
///
/// pinnacle_api::main!(config);
/// ```
#[macro_export]
macro_rules! main {
    ($func:ident) => {
        #[$crate::tokio::main(crate = "pinnacle_api::tokio")]
        async fn main() {
            $crate::config!($func);
        }
    };
}

/// Connects to Pinnacle before calling the provided async function,
/// then blocks until Pinnacle exits.
///
/// This macro is called by [`main`]. It is exposed for use in case you
/// need to change the generated main function.
///
/// # Examples
///
/// ```no_run
/// async fn config() {}
///
/// #[pinnacle_api::tokio::main(worker_threads = 8)]
/// async fn main() {
///     pinnacle_api::config!(config);
/// }
/// ```
#[macro_export]
macro_rules! config {
    ($func:ident) => {
        $crate::connect().await.unwrap();
        $func().await;
        $crate::block().await;
    };
}
