// SPDX-License-Identifier: GPL-3.0-or-later

//! A very, VERY WIP Smithay-based Wayland compositor.
//!
//! Pinnacle is heavily inspired by the [Awesome Window Manager](https://awesomewm.org),
//! and this is an attempt to make something akin to it for Wayland.
//!
//! While Pinnacle is not a library, this documentation serves to guide those who want to
//! contribute or learn how building something like this works.

// #![deny(unused_imports)] // gonna force myself to keep stuff clean
#![warn(clippy::unwrap_used)]

use clap::Parser;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt::writer::MakeWriterExt, EnvFilter};
use xdg::BaseDirectories;
use nix::unistd::Uid;

mod api;
mod backend;
mod config;
mod cursor;
mod focus;
mod grab;
mod handlers;
mod input;
mod layout;
mod output;
mod render;
mod state;
mod tag;
mod window;

lazy_static::lazy_static! {
    pub static ref XDG_BASE_DIRS: BaseDirectories =
        BaseDirectories::with_prefix("pinnacle").expect("couldn't create xdg BaseDirectories");
}

#[derive(clap::Args, Debug)]
#[group(id = "backend", required = false, multiple = false)]
struct Backends {
    #[arg(long, group = "backend")]
    /// Run Pinnacle in a window in your graphical environment
    winit: bool,
    #[arg(long, group = "backend")]
    /// Run Pinnacle from a tty
    udev: bool,
}

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    backend: Backends,
    #[arg(long)]
    /// Allow running Pinnacle as root (this is NOT recommended)
    allow_root: bool,
    #[arg(long, requires = "backend")]
    /// Force Pinnacle to run with the provided backend
    force: bool,
}

fn main() -> anyhow::Result<()> {
    let xdg_state_dir = XDG_BASE_DIRS.get_state_home();

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::HOURLY)
        .filename_suffix("pinnacle.log")
        .max_log_files(8)
        .build(xdg_state_dir)
        .expect("failed to build file logger");

    let (appender, _guard) = tracing_appender::non_blocking(appender);
    let writer = appender.and(std::io::stdout);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("debug"));

    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env_filter)
        .with_writer(writer)
        .init();

    let args = Args::parse();

    if Uid::effective().is_root() && !args.allow_root {
        println!("You are trying to run Pinnacle as root.\nThis is NOT recommended.\nTo run Pinnacle as root, pass in the --allow-root flag. Again, this is NOT recommended.");
        return Ok(());
    }

    let in_graphical_env =
        std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

    match (args.backend.winit, args.backend.udev, args.force) {
        (false, false, _) => {
            if in_graphical_env {
                tracing::info!("Starting winit backend");
                crate::backend::winit::run_winit()?;
            } else {
                tracing::info!("Starting udev backend");
                crate::backend::udev::run_udev()?;
            }
        }
        (true, false, force) => {
            if !in_graphical_env {
                if force {
                    tracing::warn!("Starting winit backend with no detected graphical environment");
                    crate::backend::winit::run_winit()?;
                } else {
                    println!("Both WAYLAND_DISPLAY and DISPLAY are not set.");
                    println!("If you are trying to run the winit backend in a tty, it won't work.");
                    println!("If you really want to, additionally pass in the --force flag.");
                }
            } else {
                tracing::info!("Starting winit backend");
                crate::backend::winit::run_winit()?;
            }
        }
        (false, true, force) => {
            if in_graphical_env {
                if force {
                    tracing::warn!("Starting udev backend with a detected graphical environment");
                    crate::backend::udev::run_udev()?;
                } else {
                    println!("WAYLAND_DISPLAY and/or DISPLAY are set.");
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
        _ => unreachable!(),
    }

    Ok(())
}
