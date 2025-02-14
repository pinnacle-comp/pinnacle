// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    io::{BufRead, BufReader},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Context;
use pinnacle::{
    cli::{self, generate_config, Cli, CliSubcommand, ConfigSubcommand},
    config::{get_config_dir, parse_startup_config, StartupConfig},
    session::{import_environment, notify_fd},
    state::State,
    util::increase_nofile_rlimit,
};
use smithay::reexports::{calloop::EventLoop, rustix::process::geteuid};
use tracing::{error, info, warn};
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use xdg::BaseDirectories;

#[cfg(feature = "tracy-alloc")]
#[global_allocator]
static GLOBAL_ALLOC: tracy_client::ProfiledAllocator<std::alloc::System> =
    tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

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
        EnvFilter::new("debug,h2=warn,hyper=warn,smithay::xwayland::xwm=warn,wgpu_hal=warn,naga=warn,wgpu_core=warn,cosmic_text=warn,iced_wgpu=warn,sctk=error");

    let file_log_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_ansi(false)
        .with_writer(appender)
        .with_filter(file_log_env_filter);

    let stdout_env_filter =
        env_filter.unwrap_or_else(|_| EnvFilter::new("warn,pinnacle=info,snowcap=info,sctk=error"));
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

    let cli = Cli::parse();

    if let Some(subcommand) = &cli.subcommand {
        match subcommand {
            CliSubcommand::Config(ConfigSubcommand::Gen(config_gen)) => {
                if let Err(err) = generate_config(config_gen.clone()) {
                    error!("Error generating config: {err}");
                }
            }
            CliSubcommand::Info => {
                let info = format!(
                    "Pinnacle, built in {opt} mode with Rust {rust_ver}\n\
                    \n\
                    Branch: {branch}{dirty}\n\
                    Commit: {commit} ({commit_msg})\n\
                    \n\
                    OS: {os}",
                    branch = env!("VERGEN_GIT_BRANCH"),
                    dirty = if env!("VERGEN_GIT_DIRTY") == "true" {
                        " (dirty)"
                    } else {
                        ""
                    },
                    commit = env!("VERGEN_GIT_SHA"),
                    commit_msg = env!("VERGEN_GIT_COMMIT_MESSAGE"),
                    opt = if env!("VERGEN_CARGO_DEBUG") == "true" {
                        "debug"
                    } else {
                        "release"
                    },
                    rust_ver = env!("VERGEN_RUSTC_SEMVER"),
                    os = env!("VERGEN_SYSINFO_OS_VERSION"),
                );

                println!("{info}");
            }
        }
        return Ok(());
    }

    tracy_client::Client::start();

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

    let session = cli.session;

    if cli.session {
        if in_graphical_env {
            warn!("You are trying to run Pinnacle as a session with WAYLAND_DISPLAY and/or DISPLAY set.");
            warn!("If you are in a graphical environment, do not continue. This will mess up the global environment.");
            warn!("If you are not, please unset WAYLAND_DISPLAY and DISPLAY and retry.");
            return Ok(());
        }

        std::env::set_var("XDG_CURRENT_DESKTOP", "pinnacle");
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
    }

    if !sysinfo::set_open_files_limit(0) {
        warn!("Unable to set `sysinfo`'s open files limit to 0");
    }

    let backend = match in_graphical_env {
        true => cli::Backend::Winit,
        false => cli::Backend::Udev,
    };

    let config_dir = cli
        .config_dir
        .clone()
        .unwrap_or_else(|| get_config_dir(&base_dirs));

    // Parse the startup config once to resolve it with CLI flags.
    // The startup config is parsed a second time when `start_config`
    // is called below which is not ideal but I'm lazy.
    let startup_config = match parse_startup_config(&config_dir) {
        Ok(startup_config) => startup_config,
        Err(err) => {
            warn!(
                "Could not load `pinnacle.toml` at {}: {err}",
                config_dir.display()
            );
            StartupConfig::default()
        }
    };

    let startup_config = startup_config.merge_and_resolve(Some(&cli), &config_dir)?;

    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;

    let mut state = State::new(
        backend,
        event_loop.handle(),
        event_loop.get_signal(),
        config_dir,
        Some(cli),
    )?;

    info!(
        "Setting WAYLAND_DISPLAY to {}",
        state.pinnacle.socket_name.to_string_lossy()
    );
    std::env::set_var("WAYLAND_DISPLAY", &state.pinnacle.socket_name);

    state
        .pinnacle
        .start_grpc_server(&startup_config.socket_dir.clone())?;

    #[cfg(feature = "snowcap")]
    {
        use tokio::sync::oneshot::error::TryRecvError;

        let (sender, mut recv) = tokio::sync::oneshot::channel();
        let join_handle = tokio::task::spawn_blocking(move || {
            let _span = tracing::error_span!("snowcap");
            let _span = _span.enter();
            snowcap::start(Some(sender));
        });

        let stop_signal = loop {
            if join_handle.is_finished() {
                panic!("snowcap failed to start");
            }
            match recv.try_recv() {
                Ok(stop_signal) => break stop_signal,
                Err(TryRecvError::Empty) => {
                    event_loop.dispatch(Duration::from_secs(1), &mut state)?;
                    state.on_event_loop_cycle_completion();
                }
                Err(TryRecvError::Closed) => panic!("snowcap failed to start"),
            }
        };

        state.pinnacle.snowcap_stop_signal = Some(stop_signal);
        state.pinnacle.snowcap_join_handle = Some(join_handle);
    }

    if !startup_config.no_xwayland {
        let finished_flag = Arc::new(AtomicBool::new(false));

        match state.pinnacle.insert_xwayland_source(finished_flag.clone()) {
            Ok(()) => {
                // Wait for xwayland to start so the config gets DISPLAY
                while !finished_flag.load(Ordering::Relaxed) {
                    event_loop.dispatch(None, &mut state)?;
                    state.on_event_loop_cycle_completion();
                }
            }
            Err(err) => error!("Failed to start xwayland: {err}"),
        }
    }

    if session {
        import_environment();
    }

    if let Err(err) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
        warn!("Error notifying systemd: {err}");
    }

    if let Err(err) = notify_fd() {
        warn!("Error norifying fd: {err}");
    }

    if !startup_config.no_config {
        state.pinnacle.start_config(false)?;
    } else {
        info!("`no-config` option was set, not spawning config");
    }

    event_loop.run(Duration::from_secs(1), &mut state, |state| {
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
