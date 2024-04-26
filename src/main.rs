// SPDX-License-Identifier: GPL-3.0-or-later

//! A very, VERY WIP Smithay-based Wayland compositor.
//!
//! Pinnacle is heavily inspired by the [Awesome Window Manager](https://awesomewm.org),
//! and this is an attempt to make something akin to it for Wayland.
//!
//! While Pinnacle is not a library, this documentation serves to guide those who want to
//! contribute or learn how building something like this works.

// #![deny(unused_imports)] // this has remained commented out for months lol
#![warn(clippy::unwrap_used)]

use std::io::{BufRead, BufReader};

use anyhow::Context;
use nix::unistd::Uid;
use pinnacle::{
    backend::{udev::setup_udev, winit::setup_winit},
    cli::{self, Cli},
};
use tracing::{error, info, warn};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use xdg::BaseDirectories;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let xdg_state_dir = BaseDirectories::with_prefix("pinnacle")?.get_state_home();

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::HOURLY)
        .filename_suffix("pinnacle.log")
        .max_log_files(8)
        .build(xdg_state_dir)
        .context("failed to build file logger")?;

    let (appender, _guard) = tracing_appender::non_blocking(appender);

    let env_filter = EnvFilter::try_from_default_env();

    let file_log_env_filter = EnvFilter::new("debug,h2=warn,smithay::xwayland::xwm=warn");

    let file_log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(false)
        .with_writer(appender)
        .with_filter(file_log_env_filter);

    let stdout_env_filter = env_filter.unwrap_or_else(|_| EnvFilter::new("warn,pinnacle=info"));
    let stdout_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(std::io::stdout)
        .with_filter(stdout_env_filter);

    tracing_subscriber::registry()
        .with(file_log_layer)
        .with(stdout_layer)
        .init();

    set_log_panic_hook();

    let Some(cli) = Cli::parse_and_prompt() else {
        return Ok(());
    };

    if Uid::effective().is_root() {
        if !cli.allow_root {
            warn!("You are trying to run Pinnacle as root.");
            warn!("This is NOT recommended.");
            warn!("To run Pinnacle as root, pass in the `--allow-root` flag.");
            warn!("Again, this is NOT recommended.");
            return Ok(());
        } else {
            warn!("Running Pinnacle as root. I hope you know what you're doing 🫡");
        }
    }

    let in_graphical_env =
        std::env::var("WAYLAND_DISPLAY").is_ok() || std::env::var("DISPLAY").is_ok();

    if !sysinfo::set_open_files_limit(0) {
        warn!("Unable to set `sysinfo`'s open files limit to 0.");
        warn!("You may see LOTS of file descriptors open under Pinnacle.");
    }

    let (mut state, mut event_loop) = match (cli.backend, cli.force) {
        (None, _) => {
            if in_graphical_env {
                info!("Starting winit backend");
                setup_winit(cli.no_config, cli.config_dir)?
            } else {
                info!("Starting udev backend");
                setup_udev(cli.no_config, cli.config_dir)?
            }
        }
        (Some(cli::Backend::Winit), force) => {
            if !in_graphical_env {
                if force {
                    warn!("Starting winit backend with no detected graphical environment");
                    setup_winit(cli.no_config, cli.config_dir)?
                } else {
                    warn!("Both WAYLAND_DISPLAY and DISPLAY are not set.");
                    warn!("If you are trying to run the winit backend in a tty, it won't work.");
                    warn!("If you really want to, additionally pass in the `--force` flag.");
                    return Ok(());
                }
            } else {
                info!("Starting winit backend");
                setup_winit(cli.no_config, cli.config_dir)?
            }
        }
        (Some(cli::Backend::Udev), force) => {
            if in_graphical_env {
                if force {
                    warn!("Starting udev backend with a detected graphical environment");
                    setup_udev(cli.no_config, cli.config_dir)?
                } else {
                    warn!("WAYLAND_DISPLAY and/or DISPLAY are set.");
                    warn!("If you are trying to run the udev backend in a graphical environment,");
                    warn!("it won't work and may mess some things up.");
                    warn!("If you really want to, additionally pass in the `--force` flag.");
                    return Ok(());
                }
            } else {
                info!("Starting udev backend");
                setup_udev(cli.no_config, cli.config_dir)?
            }
        }
    };

    event_loop.run(None, &mut state, |state| {
        state.update_pointer_focus();
        state.fixup_z_layering();
        state.pinnacle.space.refresh();
        state.pinnacle.popup_manager.cleanup();

        state
            .pinnacle
            .display_handle
            .flush_clients()
            .expect("failed to flush client buffers");

        // TODO: couple these or something, this is really error-prone
        assert_eq!(
            state.pinnacle.windows.len(),
            state.pinnacle.z_index_stack.len(),
            "Length of `windows` and `z_index_stack` are different. \
            If you see this, report it to the developer."
        );
    })?;

    Ok(())
}

/// Augment the default panic hook to attempt logging the panic message
/// using tracing. Allows the message to be written to file logs.
fn set_log_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _span = tracing::error_span!("panic");
        let _span = _span.enter();
        error!("Panic occurred! Attempting to log backtrace");
        let buffer = gag::BufferRedirect::stderr();
        if let Ok(buffer) = buffer {
            hook(info);
            let mut reader = BufReader::new(buffer).lines();
            while let Some(Ok(line)) = reader.next() {
                error!("{line}");
            }
        } else {
            error!("Attempt failed, printing normally");
            hook(info);
        }
    }));
}
