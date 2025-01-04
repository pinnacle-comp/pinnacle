use crate::{
    api::{
        input::InputService, layout::LayoutService, output::OutputService,
        pinnacle::PinnacleService, process::ProcessService, signal::SignalService, tag::TagService,
        window::WindowService, RenderService,
    },
    cli::Cli,
    output::OutputName,
    state::Pinnacle,
    tag::Tag,
    window::rules::{WindowRule, WindowRuleCondition},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::Context;
use indexmap::IndexSet;
use pinnacle_api_defs::pinnacle::{
    layout::v0alpha1::layout_service_server::LayoutServiceServer,
    process::v0alpha1::process_service_server::ProcessServiceServer,
    render::v0alpha1::render_service_server::RenderServiceServer,
};
use smithay::{
    reexports::calloop::{self, channel::Event, LoopHandle, RegistrationToken},
    utils::{Logical, Point},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinHandle,
};
use toml::Table;

use tracing::{debug, error, info, info_span, warn, Instrument};
use xdg::BaseDirectories;

use crate::{
    state::{State, WithState},
    tag::TagId,
};

const DEFAULT_SOCKET_DIR: &str = "/tmp";

mod builtin {
    include!("../api/rust/examples/default_config/main.rs");

    pub fn run() {
        main();
    }
}

const STARTUP_CONFIG_TOML_NAME: &str = "pinnacle.toml";

/// The startup config struct containing what to run, what envs to run it with, various keybinds, and
/// the target socket directory.
#[derive(serde::Deserialize, Debug, PartialEq, Default)]
pub struct StartupConfig {
    pub run: Vec<String>,
    pub envs: Option<Table>,
    pub socket_dir: Option<PathBuf>,
    pub no_config: Option<bool>,
    pub no_xwayland: Option<bool>,
}

/// A metaconfig with fields resolved.
///
/// The priority is:
/// 1. CLI options
/// 2. Startup config options
/// 3. Defaults
#[derive(Debug, PartialEq)]
pub struct ResolvedStartupConfig {
    pub run: Vec<String>,
    pub envs: Table,

    pub socket_dir: PathBuf,
    pub no_config: bool,
    pub no_xwayland: bool,
}

impl StartupConfig {
    /// Merges CLI options with this startup config.
    pub fn merge_and_resolve(
        self,
        cli: Option<&crate::cli::Cli>,
        config_dir: &Path,
    ) -> anyhow::Result<ResolvedStartupConfig> {
        let socket_dir = if let Some(socket_dir) = cli
            .and_then(|cli| cli.socket_dir.as_ref())
            .or(self.socket_dir.as_ref())
        {
            let socket_dir = shellexpand::path::full(socket_dir)?.to_path_buf();

            // cd into the metaconfig dir and canonicalize to preserve relative paths
            // like ./dir/here
            let current_dir = std::env::current_dir()?;

            std::env::set_current_dir(config_dir)?;
            let socket_dir = socket_dir.canonicalize()?;
            std::env::set_current_dir(current_dir)?;
            socket_dir
        } else {
            // Otherwise, use $XDG_RUNTIME_DIR. If that doesn't exist, use /tmp.
            BaseDirectories::with_prefix("pinnacle")?
                .get_runtime_directory()
                .cloned()
                .unwrap_or(PathBuf::from(DEFAULT_SOCKET_DIR))
        };

        Ok(ResolvedStartupConfig {
            run: self.run,
            envs: self.envs.unwrap_or_default(),
            socket_dir,
            no_config: cli
                .and_then(|cli| cli.no_config.then_some(true))
                .or(self.no_config)
                .unwrap_or_default(),
            no_xwayland: cli
                .and_then(|cli| cli.no_xwayland.then_some(true))
                .or(self.no_xwayland)
                .unwrap_or_default(),
        })
    }
}

#[cfg(feature = "testing")]
impl ResolvedStartupConfig {
    pub fn new_for_testing(no_config: bool, no_xwayland: bool) -> Self {
        ResolvedStartupConfig {
            run: vec![],
            envs: Default::default(),
            socket_dir: PathBuf::from(""),
            no_config,
            no_xwayland,
        }
    }
}

/// The current state of configuration.
#[derive(Debug)]
pub struct Config {
    /// Window rules and conditions on when those rules should apply
    pub window_rules: Vec<(WindowRuleCondition, WindowRule)>,
    /// Saved states when outputs are disconnected
    pub connector_saved_states: HashMap<OutputName, ConnectorSavedState>,

    pub config_join_handle: Option<JoinHandle<()>>,
    pub(crate) config_reload_on_crash_token: Option<RegistrationToken>,

    pub keepalive_sender: Option<tokio::sync::oneshot::Sender<()>>,

    pub config_dir: PathBuf,
    pub cli: Option<Cli>,
    socket_path: Option<PathBuf>,
}

impl Drop for Config {
    fn drop(&mut self) {
        if let Some(socket_path) = self.socket_path.as_ref() {
            let _ = std::fs::remove_file(socket_path);
        }
    }
}

impl Config {
    pub fn new(config_dir: PathBuf, cli: Option<Cli>) -> Self {
        Config {
            window_rules: Vec::new(),
            connector_saved_states: HashMap::new(),
            config_join_handle: None,
            config_reload_on_crash_token: None,
            keepalive_sender: None,
            config_dir,
            cli,
            socket_path: None,
        }
    }

    pub(crate) fn clear(&mut self, loop_handle: &LoopHandle<State>) {
        self.window_rules.clear();
        self.connector_saved_states.clear();
        if let Some(join_handle) = self.config_join_handle.take() {
            join_handle.abort();
        }
        if let Some(shutdown_sender) = self.keepalive_sender.take() {
            if shutdown_sender.send(()).is_err() {
                warn!("Failed to send shutdown signal to config");
            }
        }
        if let Some(token) = self.config_reload_on_crash_token.take() {
            loop_handle.remove(token);
        }
    }
}

/// State saved when an output is disconnected. When the output is reconnected to the same
/// connector, the saved state will apply to restore its state.
#[derive(Debug, Default, Clone)]
pub struct ConnectorSavedState {
    /// The old location
    pub loc: Point<i32, Logical>,
    /// The output's previous tags
    pub tags: IndexSet<Tag>,
    /// The output's previous scale
    pub scale: Option<smithay::output::Scale>,
    // TODO: transform
}

/// Parse a `pinnacle.toml` file in `config_dir`, if any.
pub fn parse_startup_config(config_dir: &Path) -> anyhow::Result<StartupConfig> {
    let startup_config_path = config_dir.join(STARTUP_CONFIG_TOML_NAME);

    std::fs::read_to_string(&startup_config_path)
        .with_context(|| format!("Failed to read {}", startup_config_path.display()))
        .and_then(|data| {
            toml::from_str(&data).with_context(|| {
                format!(
                    "Failed to deserialize toml in {}",
                    startup_config_path.display()
                )
            })
        })
}

/// Get the config dir. This is $PINNACLE_CONFIG_DIR, then $XDG_CONFIG_HOME/pinnacle,
/// then ~/.config/pinnacle.
pub fn get_config_dir(xdg_base_dirs: &BaseDirectories) -> PathBuf {
    let config_dir = std::env::var("PINNACLE_CONFIG_DIR")
        .ok()
        .and_then(|s| Some(PathBuf::from(shellexpand::full(&s).ok()?.to_string())));

    config_dir.unwrap_or(xdg_base_dirs.get_config_home())
}

impl Pinnacle {
    pub fn start_config(&mut self, builtin: bool) -> anyhow::Result<()> {
        // Clear state

        debug!("Clearing tags");
        for output in self.outputs.keys() {
            output.with_state_mut(|state| {
                for tag in state.tags.iter() {
                    tag.make_defunct();
                }
            });
        }

        TagId::reset();

        debug!("Clearing input state");

        self.input_state.clear();

        self.config.clear(&self.loop_handle);

        self.signal_state.clear();

        let load_default_config = |pinnacle: &mut Pinnacle, reason: &str| {
            if builtin {
                panic!("builtin rust config crashed; this is a bug and you should open an issue")
            }
            warn!(
                "Unable to load config at {}: {reason}",
                pinnacle.config.config_dir.display()
            );

            info!("Falling back to builtin Rust config");
            pinnacle.start_config(true)
        };

        let startup_config = if builtin {
            StartupConfig::default()
        } else {
            match parse_startup_config(&self.config.config_dir) {
                Ok(startup_config) => startup_config,
                Err(err) => {
                    let msg = format!(
                        "Could not load `{STARTUP_CONFIG_TOML_NAME}` at {}: {err}",
                        self.config.config_dir.display()
                    );
                    return load_default_config(self, &msg);
                }
            }
        };

        let startup_config =
            startup_config.merge_and_resolve(self.config.cli.as_ref(), &self.config.config_dir)?;

        if startup_config.no_config {
            info!("`no-config` option was set, not spawning config");
            return Ok(());
        }

        if builtin {
            let (pinger, ping_source) = calloop::ping::make_ping()?;

            let token = self
                .loop_handle
                .insert_source(ping_source, move |_, _, _state| {
                    panic!("builtin rust config crashed; this is a bug");
                })?;

            std::thread::spawn(move || {
                info!("Starting builtin Rust config");
                builtin::run();
                pinger.ping();
            });

            self.config.config_reload_on_crash_token = Some(token);
        } else {
            let config_dir = &self.config.config_dir;
            let command = startup_config.run.clone();
            let mut command_iter = command.iter();

            let arg0 = match command_iter.next() {
                Some(arg0) => arg0,
                None => return load_default_config(self, "no command specified"),
            };

            let command_rest = command_iter.collect::<Vec<_>>();

            debug!(arg0, ?command_rest);

            let envs = startup_config
                .envs
                .clone()
                .into_iter()
                .map(|(key, val)| -> anyhow::Result<Option<(String, String)>> {
                    if let toml::Value::String(string) = val {
                        Ok(Some((key, shellexpand::full(&string)?.to_string())))
                    } else {
                        Ok(None)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten();

            debug!("Config envs are {envs:?}");

            info!(
                "Starting config process at {} with {:?}",
                config_dir.display(),
                command
            );

            let mut cmd = tokio::process::Command::new(arg0);
            cmd.args(command_rest)
                .envs(envs)
                .current_dir(config_dir)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);

            let mut child = match cmd.spawn() {
                Ok(child) => child,
                Err(err) => {
                    return load_default_config(
                        self,
                        &format!("failed to start config process {cmd:?}: {err}"),
                    )
                }
            };

            if let Some(stdout) = child.stdout.take() {
                let mut reader = BufReader::new(stdout).lines();
                tokio::spawn(
                    async move {
                        while let Ok(Some(line)) = reader.next_line().await {
                            match line.split_whitespace().next() {
                                Some("WARN") => warn!("{line}"),
                                Some("ERROR" | "FATAL") => error!("{line}"),
                                Some("DEBUG") => debug!("{line}"),
                                _ => info!("{line}"),
                            }
                        }
                    }
                    .instrument(info_span!("config_stdout")),
                );
            }

            if let Some(stderr) = child.stderr.take() {
                let mut reader = BufReader::new(stderr).lines();
                tokio::spawn(
                    async move {
                        while let Ok(Some(line)) = reader.next_line().await {
                            match line.split_whitespace().next() {
                                Some("WARN") => warn!("{line}"),
                                Some("ERROR" | "FATAL") => error!("{line}"),
                                Some("DEBUG") => debug!("{line}"),
                                _ => info!("{line}"),
                            }
                        }
                    }
                    .instrument(info_span!("config_stderr")),
                );
            }

            info!("Started config with {:?}", command);

            let (pinger, ping_source) = calloop::ping::make_ping()?;

            let token = self
                .loop_handle
                .insert_source(ping_source, move |_, _, state| {
                    error!("Config crashed! Falling back to default config");
                    state
                        .pinnacle
                        .start_config(true)
                        .expect("failed to start default config");
                })?;

            self.config.config_join_handle = Some(tokio::spawn(async move {
                let _ = child.wait().await;
                pinger.ping();
            }));

            self.config.config_reload_on_crash_token = Some(token);
        }

        Ok(())
    }

    pub fn start_grpc_server(&mut self, socket_dir: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(socket_dir)?;

        let socket_name = format!("pinnacle-grpc-{}.sock", std::process::id());

        let socket_path = socket_dir.join(socket_name);

        if let Ok(true) = socket_path.try_exists() {
            std::fs::remove_file(&socket_path)
                .context(format!("Failed to remove old socket at {socket_path:?}"))?;
        }

        std::env::set_var(
            "PINNACLE_PROTO_DIR",
            self.xdg_base_dirs.get_data_file("protobuf"),
        );

        let (grpc_sender, grpc_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce(&mut State) + Send>>();

        self.loop_handle
            .insert_source(grpc_receiver, |msg, _, state| match msg {
                Event::Msg(f) => f(state),
                Event::Closed => error!("grpc receiver was closed"),
            })
            .expect("failed to insert grpc_receiver into loop");

        let pinnacle_service = PinnacleService::new(grpc_sender.clone());
        let input_service = InputService::new(grpc_sender.clone());
        let process_service = ProcessService::new(grpc_sender.clone());
        let tag_service = TagService::new(grpc_sender.clone());
        let output_service = OutputService::new(grpc_sender.clone());
        let window_service = WindowService::new(grpc_sender.clone());
        let signal_service = SignalService::new(grpc_sender.clone());
        let layout_service = LayoutService::new(grpc_sender.clone());
        let render_service = RenderService::new(grpc_sender.clone());

        let refl_service = tonic_reflection::server::Builder::configure()
            .register_encoded_file_descriptor_set(pinnacle_api_defs::FILE_DESCRIPTOR_SET)
            .build_v1()?;

        let uds = tokio::net::UnixListener::bind(&socket_path)?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

        std::env::set_var("PINNACLE_GRPC_SOCKET", &socket_path);

        let grpc_server = tonic::transport::Server::builder()
            .add_service(refl_service)
            .add_service(pinnacle_api_defs::pinnacle::v1::pinnacle_service_server::PinnacleServiceServer::new(pinnacle_service))
            .add_service(pinnacle_api_defs::pinnacle::window::v1::window_service_server::WindowServiceServer::new(window_service))
            .add_service(pinnacle_api_defs::pinnacle::tag::v1::tag_service_server::TagServiceServer::new(tag_service))
            .add_service(pinnacle_api_defs::pinnacle::output::v1::output_service_server::OutputServiceServer::new(output_service))
            .add_service(pinnacle_api_defs::pinnacle::input::v1::input_service_server::InputServiceServer::new(input_service))
            .add_service(pinnacle_api_defs::pinnacle::process::v1::process_service_server::ProcessServiceServer::new(process_service))
            .add_service(pinnacle_api_defs::pinnacle::signal::v1::signal_service_server::SignalServiceServer::new(signal_service))
            .add_service(LayoutServiceServer::new(layout_service))
            .add_service(RenderServiceServer::new(render_service));

        self.grpc_server_join_handle = Some(tokio::spawn(async move {
            if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                error!("gRPC server error: {err}");
            }
        }));

        info!("gRPC server started at {}", socket_path.display());

        self.config.socket_path = Some(socket_path);

        Ok(())
    }
}

// FIXME: update tests
#[cfg(test)]
mod tests {
    use super::*;
    use std::env::var;

    #[test]
    fn get_config_dir_with_relative_env_works() -> anyhow::Result<()> {
        let relative_path = "api/rust/examples/default_config";

        temp_env::with_var("PINNACLE_CONFIG_DIR", Some(relative_path), || {
            let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;

            // Prepending the relative path with the current dir *shouldn't* be necessary, me thinks
            let expected = PathBuf::from(relative_path);

            assert_eq!(get_config_dir(&xdg_base_dirs), expected);

            Ok(())
        })
    }

    #[test]
    fn get_config_dir_with_tilde_env_works() -> anyhow::Result<()> {
        temp_env::with_var("PINNACLE_CONFIG_DIR", Some("~/some/dir/somewhere/"), || {
            let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;
            let expected = PathBuf::from(var("HOME")?).join("some/dir/somewhere");

            assert_eq!(get_config_dir(&xdg_base_dirs), expected);

            Ok(())
        })
    }

    #[test]
    fn get_config_dir_with_absolute_env_works() -> anyhow::Result<()> {
        let absolute_path = "/its/morbin/time";

        temp_env::with_var("PINNACLE_CONFIG_DIR", Some(absolute_path), || {
            let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;
            let expected = PathBuf::from(absolute_path);

            assert_eq!(get_config_dir(&xdg_base_dirs), expected);

            Ok(())
        })
    }

    #[test]
    fn get_config_dir_without_env_and_with_xdg_works() -> anyhow::Result<()> {
        let xdg_config_home = "/some/different/xdg/config/path";

        temp_env::with_vars(
            [
                ("PINNACLE_CONFIG_DIR", None),
                ("XDG_CONFIG_HOME", Some(xdg_config_home)),
            ],
            || {
                let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;
                let expected = PathBuf::from(xdg_config_home).join("pinnacle");

                assert_eq!(get_config_dir(&xdg_base_dirs), expected);

                Ok(())
            },
        )
    }

    #[test]
    fn get_config_dir_without_env_and_without_xdg_works() -> anyhow::Result<()> {
        temp_env::with_vars(
            [
                ("PINNACLE_CONFIG_DIR", None::<&str>),
                ("XDG_CONFIG_HOME", None),
            ],
            || {
                let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;
                let expected = PathBuf::from(var("HOME")?).join(".config/pinnacle");

                assert_eq!(get_config_dir(&xdg_base_dirs), expected);

                Ok(())
            },
        )
    }

    #[test]
    fn full_startup_config_successfully_parses() -> anyhow::Result<()> {
        let startup_config_text = r#"
            run = ["lua", "init.lua"]

            socket_dir = "/path/to/socket/dir"

            no_config = true
            no_xwayland = true

            [envs]
            MARCO = "polo"
            SUN = "chips"
        "#;

        let config_dir = tempfile::tempdir()?;
        std::fs::write(
            config_dir.path().join(STARTUP_CONFIG_TOML_NAME),
            startup_config_text,
        )?;

        let expected_startup_config = StartupConfig {
            run: vec!["lua".to_string(), "init.lua".to_string()],
            envs: Some(toml::Table::from_iter([
                ("MARCO".to_string(), toml::Value::String("polo".to_string())),
                ("SUN".to_string(), toml::Value::String("chips".to_string())),
            ])),
            socket_dir: Some("/path/to/socket/dir".into()),
            no_config: Some(true),
            no_xwayland: Some(true),
        };

        assert_eq!(
            parse_startup_config(config_dir.path())?,
            expected_startup_config
        );

        Ok(())
    }

    #[test]
    fn minimal_startup_config_successfully_parses() -> anyhow::Result<()> {
        let startup_config_text = r#"
            run = ["lua", "init.lua"]
        "#;

        let startup_config_dir = tempfile::tempdir()?;
        std::fs::write(
            startup_config_dir.path().join(STARTUP_CONFIG_TOML_NAME),
            startup_config_text,
        )?;

        let expected_startup_config = StartupConfig {
            run: vec!["lua".to_string(), "init.lua".to_string()],
            envs: None,
            socket_dir: None,
            no_config: None,
            no_xwayland: None,
        };

        assert_eq!(
            parse_startup_config(startup_config_dir.path())?,
            expected_startup_config
        );

        Ok(())
    }

    #[test]
    fn incorrect_startup_config_does_not_parse() -> anyhow::Result<()> {
        let startup_config_text = r#"
            run = "lua" # not an array
        "#;

        let config_dir = tempfile::tempdir()?;
        std::fs::write(
            config_dir.path().join("metaconfig.toml"),
            startup_config_text,
        )?;

        assert!(parse_startup_config(config_dir.path()).is_err());

        Ok(())
    }

    // TODO: test for error if `run` isn't present
}
