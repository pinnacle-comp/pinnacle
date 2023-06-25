// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! A very, VERY WIP Smithay-based Wayland compositor.
//!
//! Pinnacle is heavily inspired by the [Awesome Window Manager](https://awesomewm.org),
//! and this is an attempt to make something akin to it for Wayland.
//!
//! While Pinnacle is not a library, this documentation serves to guide those who want to
//! contribute or learn how building something like this works.

// test

mod api;
mod backend;
mod cursor;
mod focus;
mod grab;
mod handlers;
mod input;
mod layout;
mod pointer;
mod render;
mod state;
mod tag;
mod window;
mod xdg;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    match tracing_subscriber::EnvFilter::try_from_default_env() {
        Ok(env_filter) => {
            tracing_subscriber::fmt()
                .compact()
                .with_env_filter(env_filter)
                .init();
        }
        Err(_) => {
            tracing_subscriber::fmt().compact().init();
        }
    }

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--winit") => {
            tracing::info!("Starting winit backend");
            crate::backend::winit::run_winit()?;
        }
        Some("--udev") => {
            tracing::info!("Starting udev backend");
            crate::backend::udev::run_udev()?;
        }
        Some(arg) => tracing::error!("Unknown argument {}", arg),
        None => {
            println!(
                "Specify a backend:\n\t--udev to launch Pinnacle in a tty, or\n\t--winit to launch Pinnacle as a window in your graphical environment."
            );
        }
    }

    Ok(())
}
