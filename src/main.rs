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
use pinnacle::{
    cli::{self, Cli},
    config::{get_config_dir, parse_metaconfig, Metaconfig},
    state::State,
    util::increase_nofile_rlimit,
};
use smithay::reexports::{
    calloop::{self, EventLoop},
    rustix::process::geteuid,
};
use tracing::{error, info, warn};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use xdg::BaseDirectories;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_dirs = BaseDirectories::with_prefix("pinnacle")?;
    let xdg_state_dir = base_dirs.get_state_home();

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(Rotation::HOURLY)
        .filename_suffix("pinnacle.log")
        .max_log_files(8)
        .build(xdg_state_dir)
        .context("failed to build file logger")?;

    let (appender, _guard) = tracing_appender::non_blocking(appender);

    let env_filter = EnvFilter::try_from_default_env();

    let file_log_env_filter =
        EnvFilter::new("debug,h2=warn,hyper=warn,smithay::xwayland::xwm=warn,wgpu_hal=warn,naga=warn,wgpu_core=warn,cosmic_text=warn,iced_wgpu=warn,sctk=warn");

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

    info!("Starting Pinnacle (commit {})", env!("VERGEN_GIT_SHA"));

    increase_nofile_rlimit();

    set_log_panic_hook();

    let Some(cli) = Cli::parse_and_prompt() else {
        return Ok(());
    };

    if geteuid().is_root() {
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

    let backend: cli::Backend = match (cli.backend, cli.force) {
        (None, _) => {
            if in_graphical_env {
                cli::Backend::Winit
            } else {
                cli::Backend::Udev
            }
        }
        (Some(cli::Backend::Winit), force) => {
            if !in_graphical_env {
                if force {
                    warn!("Starting winit backend with no detected graphical environment");
                    cli::Backend::Winit
                } else {
                    warn!("Both WAYLAND_DISPLAY and DISPLAY are not set.");
                    warn!("If you are trying to run the winit backend in a tty, it won't work.");
                    warn!("If you really want to, additionally pass in the `--force` flag.");
                    return Ok(());
                }
            } else {
                cli::Backend::Winit
            }
        }
        (Some(cli::Backend::Udev), force) => {
            if in_graphical_env {
                if force {
                    warn!("Starting udev backend with a detected graphical environment");
                    cli::Backend::Udev
                } else {
                    warn!("WAYLAND_DISPLAY and/or DISPLAY are set.");
                    warn!("If you are trying to run the udev backend in a graphical environment,");
                    warn!("it won't work and may mess some things up.");
                    warn!("If you really want to, additionally pass in the `--force` flag.");
                    return Ok(());
                }
            } else {
                cli::Backend::Udev
            }
        }
        #[cfg(feature = "testing")]
        (Some(cli::Backend::Dummy), _) => cli::Backend::Dummy,
    };

    let config_dir = cli
        .config_dir
        .clone()
        .unwrap_or_else(|| get_config_dir(&base_dirs));

    // Parse the metaconfig once to resolve it with CLI flags.
    // The metaconfig is parsed a second time when `start_config`
    // is called below which is not ideal but I'm lazy.
    let metaconfig = match parse_metaconfig(&config_dir) {
        Ok(metaconfig) => metaconfig,
        Err(err) => {
            warn!(
                "Could not load `metaconfig.toml` at {}: {err}",
                config_dir.display()
            );
            Metaconfig::default()
        }
    };

    let metaconfig = metaconfig.merge_and_resolve(Some(&cli), &config_dir)?;

    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;

    let mut state = State::new(
        backend,
        event_loop.handle(),
        event_loop.get_signal(),
        config_dir,
        Some(cli),
    )?;

    state
        .pinnacle
        .start_grpc_server(&metaconfig.socket_dir.clone())?;

    #[cfg(feature = "snowcap")]
    {
        let (ping, source) = calloop::ping::make_ping()?;
        tokio::task::spawn_blocking(move || {
            snowcap::start(Some(source));
        });
        state.pinnacle.snowcap_shutdown_ping = Some(ping);
    }

    if !metaconfig.no_xwayland {
        match state.pinnacle.insert_xwayland_source() {
            Ok(()) => {
                // Wait for xwayland to start so the config gets DISPLAY
                while state.pinnacle.xdisplay.is_none() {
                    event_loop.dispatch(None, &mut state)?;
                    state.on_event_loop_cycle_completion();
                }
            }
            Err(err) => error!("Failed to start xwayland: {err}"),
        }
    }

    if !metaconfig.no_config {
        state.pinnacle.start_config(false)?;
    } else {
        info!("`no-config` option was set, not spawning config");
    }

    event_loop.run(None, &mut state, |state| {
        state.on_event_loop_cycle_completion();
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
