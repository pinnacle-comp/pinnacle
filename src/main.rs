// SPDX-License-Identifier: GPL-3.0-or-later

//! A very, VERY WIP Smithay-based Wayland compositor.
//!
//! Pinnacle is heavily inspired by the [Awesome Window Manager](https://awesomewm.org),
//! and this is an attempt to make something akin to it for Wayland.
//!
//! While Pinnacle is not a library, this documentation serves to guide those who want to
//! contribute or learn how building something like this works.

#![deny(unused_imports)] // gonna force myself to keep stuff clean
#![warn(clippy::unwrap_used)]

mod api;
mod backend;
mod cursor;
mod focus;
mod grab;
mod handlers;
mod input;
mod layout;
mod output;
mod pointer;
mod render;
mod state;
mod tag;
mod window;

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

    let in_graphical_env =
        std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--winit") => {
            if !in_graphical_env {
                if let Some("--force") = args.next().as_deref() {
                    tracing::info!("Starting winit backend with no detected graphical environment");
                    crate::backend::winit::run_winit()?;
                } else {
                    println!("Both WAYLAND_DISPLAY and DISPLAY were not set.");
                    println!("If you are trying to run the winit backend in a tty, it won't work.");
                    println!("If you really want to, additionally pass in the --force flag.");
                }
            } else {
                tracing::info!("Starting winit backend");
                crate::backend::winit::run_winit()?;
            }
        }
        Some("--udev") => {
            if in_graphical_env {
                if let Some("--force") = args.next().as_deref() {
                    tracing::info!("Starting udev backend with a detected graphical environment");
                    crate::backend::udev::run_udev()?;
                } else {
                    println!("WAYLAND_DISPLAY and/or DISPLAY were set.");
                    println!(
                        "If you are trying to run the udev backend in a graphical environment,"
                    );
                    println!("it won't work and may mess some things up.");
                    println!("If you really want to, additionally pass in the --force flag.");
                }
            } else {
                tracing::info!("Starting udev backend");
                crate::backend::udev::run_udev()?;
            }
        }
        Some(arg) => tracing::error!("Unknown argument {}", arg),
        None => {
            if in_graphical_env {
                tracing::info!("Starting winit backend");
                crate::backend::winit::run_winit()?;
            } else {
                tracing::info!("Starting udev backend");
                crate::backend::udev::run_udev()?;
            }
        }
    }

    Ok(())
}
