use crate::{
    api::{
        layout::LayoutService, signal::SignalService, window::WindowService, InputService,
        OutputService, PinnacleService, ProcessService, RenderService, TagService,
    },
    input::ModifierMask,
    output::OutputName,
    tag::Tag,
    window::rules::{WindowRule, WindowRuleCondition},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
};

use anyhow::Context;
use pinnacle_api_defs::pinnacle::{
    input::v0alpha1::input_service_server::InputServiceServer,
    layout::v0alpha1::layout_service_server::LayoutServiceServer,
    output::v0alpha1::output_service_server::OutputServiceServer,
    process::v0alpha1::process_service_server::ProcessServiceServer,
    render::v0alpha1::render_service_server::RenderServiceServer,
    signal::v0alpha1::signal_service_server::SignalServiceServer,
    tag::v0alpha1::tag_service_server::TagServiceServer,
    v0alpha1::{pinnacle_service_server::PinnacleServiceServer, ShutdownWatchResponse},
    window::v0alpha1::window_service_server::WindowServiceServer,
};
use smithay::{
    input::keyboard::keysyms,
    reexports::calloop::{self, channel::Event, LoopHandle, RegistrationToken},
    utils::{Logical, Point},
};
use sysinfo::ProcessRefreshKind;
use tokio::task::JoinHandle;
use toml::Table;

use tracing::{debug, error, info};
use xdg::BaseDirectories;
use xkbcommon::xkb::Keysym;

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

    pub const METACONFIG: &str =
        include_str!("../api/rust/examples/default_config/metaconfig.toml");
}

/// The metaconfig struct containing what to run, what envs to run it with, various keybinds, and
/// the target socket directory.
#[derive(serde::Deserialize, Debug, PartialEq)]
pub struct Metaconfig {
    pub command: Vec<String>,
    pub envs: Option<Table>,
    pub reload_keybind: Keybind,
    pub kill_keybind: Keybind,
    pub socket_dir: Option<String>,
}

#[derive(serde::Deserialize, Debug, PartialEq)]
pub struct Keybind {
    modifiers: Vec<Modifier>,
    key: Key,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq)]
enum Modifier {
    Shift,
    Ctrl,
    Alt,
    Super,
}

// TODO: refactor metaconfig input
impl From<Vec<self::Modifier>> for ModifierMask {
    fn from(mods: Vec<self::Modifier>) -> Self {
        let mut mask = ModifierMask::empty();

        for m in mods {
            match m {
                Modifier::Shift => mask |= ModifierMask::SHIFT,
                Modifier::Ctrl => mask |= ModifierMask::CTRL,
                Modifier::Alt => mask |= ModifierMask::ALT,
                Modifier::Super => mask |= ModifierMask::SUPER,
            }
        }

        mask
    }
}

// TODO: accept xkbcommon names instead
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum Key {
    A = keysyms::KEY_a,
    B = keysyms::KEY_b,
    C = keysyms::KEY_c,
    D = keysyms::KEY_d,
    E = keysyms::KEY_e,
    F = keysyms::KEY_f,
    G = keysyms::KEY_g,
    H = keysyms::KEY_h,
    I = keysyms::KEY_i,
    J = keysyms::KEY_j,
    K = keysyms::KEY_k,
    L = keysyms::KEY_l,
    M = keysyms::KEY_m,
    N = keysyms::KEY_n,
    O = keysyms::KEY_o,
    P = keysyms::KEY_p,
    Q = keysyms::KEY_q,
    R = keysyms::KEY_r,
    S = keysyms::KEY_s,
    T = keysyms::KEY_t,
    U = keysyms::KEY_u,
    V = keysyms::KEY_v,
    W = keysyms::KEY_w,
    X = keysyms::KEY_x,
    Y = keysyms::KEY_y,
    Z = keysyms::KEY_z,
    #[serde(alias = "0")]
    Zero = keysyms::KEY_0,
    #[serde(alias = "1")]
    One = keysyms::KEY_1,
    #[serde(alias = "2")]
    Two = keysyms::KEY_2,
    #[serde(alias = "3")]
    Three = keysyms::KEY_3,
    #[serde(alias = "4")]
    Four = keysyms::KEY_4,
    #[serde(alias = "5")]
    Five = keysyms::KEY_5,
    #[serde(alias = "6")]
    Six = keysyms::KEY_6,
    #[serde(alias = "7")]
    Seven = keysyms::KEY_7,
    #[serde(alias = "8")]
    Eight = keysyms::KEY_8,
    #[serde(alias = "9")]
    Nine = keysyms::KEY_9,
    #[serde(alias = "num0")]
    NumZero = keysyms::KEY_KP_0,
    #[serde(alias = "num1")]
    NumOne = keysyms::KEY_KP_1,
    #[serde(alias = "num2")]
    NumTwo = keysyms::KEY_KP_2,
    #[serde(alias = "num3")]
    NumThree = keysyms::KEY_KP_3,
    #[serde(alias = "num4")]
    NumFour = keysyms::KEY_KP_4,
    #[serde(alias = "num5")]
    NumFive = keysyms::KEY_KP_5,
    #[serde(alias = "num6")]
    NumSix = keysyms::KEY_KP_6,
    #[serde(alias = "num7")]
    NumSeven = keysyms::KEY_KP_7,
    #[serde(alias = "num8")]
    NumEight = keysyms::KEY_KP_8,
    #[serde(alias = "num9")]
    NumNine = keysyms::KEY_KP_9,
    #[serde(alias = "esc")]
    Escape = keysyms::KEY_Escape,
}

/// The current state of configuration.
#[derive(Default, Debug)]
pub struct Config {
    /// Window rules and conditions on when those rules should apply
    pub window_rules: Vec<(WindowRuleCondition, WindowRule)>,
    /// Saved states when outputs are disconnected
    pub connector_saved_states: HashMap<OutputName, ConnectorSavedState>,

    pub config_join_handle: Option<JoinHandle<()>>,
    config_reload_on_crash_token: Option<RegistrationToken>,

    pub shutdown_sender:
        Option<tokio::sync::mpsc::UnboundedSender<Result<ShutdownWatchResponse, tonic::Status>>>,

    pub no_config: bool,
    config_dir: Option<PathBuf>,
}

impl Config {
    pub fn new(no_config: bool, config_dir: Option<PathBuf>) -> Self {
        Config {
            no_config,
            config_dir,
            ..Default::default()
        }
    }

    pub fn dir(&self, xdg_base_dirs: &BaseDirectories) -> PathBuf {
        self.config_dir
            .clone()
            .unwrap_or_else(|| get_config_dir(xdg_base_dirs))
    }

    fn clear(&mut self, loop_handle: &LoopHandle<State>) {
        self.window_rules.clear();
        self.connector_saved_states.clear();
        if let Some(join_handle) = self.config_join_handle.take() {
            join_handle.abort();
        }
        if let Some(shutdown_sender) = self.shutdown_sender.take() {
            shutdown_sender
                .send(Ok(
                    pinnacle_api_defs::pinnacle::v0alpha1::ShutdownWatchResponse {},
                ))
                .expect("failed to send shutdown signal to config");
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
    pub tags: Vec<Tag>,
    /// The output's previous scale
    pub scale: Option<smithay::output::Scale>,
}

/// Parse a metaconfig file in `config_dir`, if any.
fn parse_metaconfig(config_dir: &Path) -> anyhow::Result<Metaconfig> {
    let metaconfig_path = config_dir.join("metaconfig.toml");

    std::fs::read_to_string(&metaconfig_path)
        .with_context(|| format!("Failed to read {}", metaconfig_path.display()))
        .and_then(|data| {
            toml::from_str(&data).with_context(|| {
                format!(
                    "Failed to deserialize toml in {}",
                    metaconfig_path.display()
                )
            })
        })
}

/// Get the config dir. This is $PINNACLE_CONFIG_DIR, then $XDG_CONFIG_HOME/pinnacle,
/// then ~/.config/pinnacle.
fn get_config_dir(xdg_base_dirs: &BaseDirectories) -> PathBuf {
    let config_dir = std::env::var("PINNACLE_CONFIG_DIR")
        .ok()
        .and_then(|s| Some(PathBuf::from(shellexpand::full(&s).ok()?.to_string())));

    config_dir.unwrap_or(xdg_base_dirs.get_config_home())
}

impl State {
    /// Start the config in `config_dir`.
    ///
    /// If this method is called while a config is already running, it will be replaced.
    ///
    /// If `config_dir` is `None`, the builtin Rust config will be used.
    pub fn start_config(&mut self, mut config_dir: Option<impl AsRef<Path>>) -> anyhow::Result<()> {
        if let Some(shutdown_sender) = self.config.shutdown_sender.take() {
            shutdown_sender
                .send(Ok(
                    pinnacle_api_defs::pinnacle::v0alpha1::ShutdownWatchResponse {},
                ))
                .expect("failed to send shutdown signal to config");
        }

        let config_dir_clone = config_dir.as_ref().map(|dir| dir.as_ref().to_path_buf());
        let load_default_config = |state: &mut State, reason: &str| {
            match &config_dir_clone {
                Some(dir) => error!("Unable to load config at {}: {reason}", dir.display()),
                None => panic!(
                    "builtin rust config crashed; this is a bug and you should open an issue"
                ),
            }

            info!("Falling back to builtin Rust config");
            state.start_config(None::<PathBuf>)
        };

        // If `--no-config` was set, still load the keybinds from the default metaconfig
        if self.config.no_config {
            config_dir = None;
        }

        let metaconfig = match &config_dir {
            Some(dir) => match parse_metaconfig(dir.as_ref()) {
                Ok(metaconfig) => metaconfig,
                Err(err) => {
                    return load_default_config(self, &format!("{}, {}", err, err.root_cause()));
                }
            },
            None => toml::from_str(builtin::METACONFIG)
                .expect("builtin metaconfig was malformed; this is a bug"),
        };

        // Clear state

        debug!("Clearing tags");
        for output in self.space.outputs() {
            output.with_state_mut(|state| state.tags.clear());
        }

        TagId::reset();

        debug!("Clearing input state");

        self.input_state.clear();

        self.config.clear(&self.loop_handle);

        self.signal_state.clear();

        let reload_keybind = metaconfig.reload_keybind;
        let kill_keybind = metaconfig.kill_keybind;

        let reload_mask = ModifierMask::from(reload_keybind.modifiers);
        let kill_mask = ModifierMask::from(kill_keybind.modifiers);

        let reload_keybind = (reload_mask, Keysym::from(reload_keybind.key as u32));
        let kill_keybind = (kill_mask, Keysym::from(kill_keybind.key as u32));

        self.input_state.reload_keybind = Some(reload_keybind);
        self.input_state.kill_keybind = Some(kill_keybind);

        if self.config.no_config {
            info!("`--no-config` was set, not spawning config");
            return Ok(());
        }

        // Because the grpc server is implemented to only start once,
        // any updates to `socket_dir` won't be applied until restart.
        if self.grpc_server_join_handle.is_none() {
            // If a socket is provided in the metaconfig, use it.
            let socket_dir = if let Some(socket_dir) = &metaconfig.socket_dir {
                let Some(config_dir) = &config_dir else {
                    panic!("builtin config should not have `socket_dir` set");
                };
                let socket_dir = shellexpand::full(socket_dir)?.to_string();

                // cd into the metaconfig dir and canonicalize to preserve relative paths
                // like ./dir/here
                let current_dir = std::env::current_dir()?;

                std::env::set_current_dir(config_dir)?;
                let socket_dir = PathBuf::from(socket_dir).canonicalize()?;
                std::env::set_current_dir(current_dir)?;
                socket_dir
            } else {
                // Otherwise, use $XDG_RUNTIME_DIR. If that doesn't exist, use /tmp.
                self.xdg_base_dirs
                    .get_runtime_directory()
                    .cloned()
                    .unwrap_or(PathBuf::from(DEFAULT_SOCKET_DIR))
            };

            self.start_grpc_server(socket_dir.as_path())?;
        }

        match &config_dir {
            Some(config_dir) => {
                let config_dir = config_dir.as_ref();
                let mut command = metaconfig.command.iter();

                let arg0 = match command.next() {
                    Some(arg0) => arg0,
                    None => return load_default_config(self, "no command specified"),
                };

                let command = command.collect::<Vec<_>>();

                debug!(arg0, ?command);

                let envs = metaconfig
                    .envs
                    .unwrap_or(toml::map::Map::new())
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
                    metaconfig.command
                );

                let mut cmd = tokio::process::Command::new(arg0);
                cmd.args(command)
                    .envs(envs)
                    .current_dir(config_dir)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
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

                info!("Started config with {:?}", metaconfig.command);

                let (pinger, ping_source) = calloop::ping::make_ping()?;

                let token = self
                    .loop_handle
                    .insert_source(ping_source, move |_, _, state| {
                        error!("Config crashed! Falling back to default Lua config");
                        state
                            .start_config(None::<PathBuf>)
                            .expect("failed to start default config");
                    })?;

                self.config.config_join_handle = Some(tokio::spawn(async move {
                    let _ = child.wait().await;
                    pinger.ping();
                }));

                self.config.config_reload_on_crash_token = Some(token);
            }
            None => {
                let (pinger, ping_source) = calloop::ping::make_ping()?;

                let token = self
                    .loop_handle
                    .insert_source(ping_source, move |_, _, _state| {
                        panic!("builtin rust config crashed; this is a bug");
                    })?;

                self.config.config_join_handle = Some(tokio::task::spawn_blocking(move || {
                    info!("Starting builtin Rust config");
                    builtin::run();
                    pinger.ping();
                }));

                self.config.config_reload_on_crash_token = Some(token);
            }
        }

        Ok(())
    }

    pub fn start_grpc_server(&mut self, socket_dir: &Path) -> anyhow::Result<()> {
        self.system_processes
            .refresh_processes_specifics(ProcessRefreshKind::new());

        let multiple_instances = self
            .system_processes
            .processes_by_exact_name("pinnacle")
            .filter(|proc| proc.thread_kind().is_none())
            .count()
            > 1;

        std::fs::create_dir_all(socket_dir)?;

        let socket_name = if multiple_instances {
            let mut suffix: u8 = 1;
            while let Ok(true) = socket_dir
                .join(format!("pinnacle-grpc-{suffix}.sock"))
                .try_exists()
            {
                suffix += 1;
            }
            format!("pinnacle-grpc-{suffix}.sock")
        } else {
            "pinnacle-grpc.sock".to_string()
        };

        let socket_path = socket_dir.join(socket_name);

        // If there are multiple instances, don't touch other sockets
        if multiple_instances {
            if let Ok(true) = socket_path.try_exists() {
                std::fs::remove_file(&socket_path)
                    .context(format!("Failed to remove old socket at {socket_path:?}"))?;
            }
        } else {
            // If there aren't, remove them all
            for file in std::fs::read_dir(socket_dir)?
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("pinnacle-grpc")
                })
            {
                debug!("Removing socket at {:?}", file.path());
                std::fs::remove_file(file.path())
                    .context(format!("Failed to remove old socket at {:?}", file.path()))?;
            }
        }

        std::env::set_var(
            "PINNACLE_PROTO_DIR",
            self.xdg_base_dirs.get_data_file("protobuf"),
        );

        let (grpc_sender, grpc_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce(&mut Self) + Send>>();

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
            .build()?;

        let uds = tokio::net::UnixListener::bind(&socket_path)?;
        let uds_stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

        std::env::set_var("PINNACLE_GRPC_SOCKET", socket_path);

        let grpc_server = tonic::transport::Server::builder()
            .add_service(refl_service)
            .add_service(PinnacleServiceServer::new(pinnacle_service))
            .add_service(InputServiceServer::new(input_service))
            .add_service(ProcessServiceServer::new(process_service))
            .add_service(TagServiceServer::new(tag_service))
            .add_service(OutputServiceServer::new(output_service))
            .add_service(WindowServiceServer::new(window_service))
            .add_service(SignalServiceServer::new(signal_service))
            .add_service(LayoutServiceServer::new(layout_service))
            .add_service(RenderServiceServer::new(render_service));

        match self.xdisplay.as_ref() {
            Some(_) => {
                self.grpc_server_join_handle = Some(tokio::spawn(async move {
                    if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                        error!("gRPC server error: {err}");
                    }
                }));
            }
            // FIXME: Not really high priority but if you somehow reload the config really, REALLY
            // |      fast at startup then I think there's a chance that the gRPC server
            // |      could get started twice.
            None => self.schedule(
                |state| state.xdisplay.is_some(),
                move |state| {
                    state.grpc_server_join_handle = Some(tokio::spawn(async move {
                        if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                            error!("gRPC server error: {err}");
                        }
                    }));
                },
            ),
        }

        Ok(())
    }
}

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
    fn full_metaconfig_successfully_parses() -> anyhow::Result<()> {
        let metaconfig_text = r#"
            command = ["lua", "init.lua"]

            reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
            kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }

            socket_dir = "/path/to/socket/dir"

            [envs]
            MARCO = "polo"
            SUN = "chips"
        "#;

        let metaconfig_dir = tempfile::tempdir()?;
        std::fs::write(
            metaconfig_dir.path().join("metaconfig.toml"),
            metaconfig_text,
        )?;

        let expected_metaconfig = Metaconfig {
            command: vec!["lua".to_string(), "init.lua".to_string()],
            envs: Some(toml::Table::from_iter([
                ("MARCO".to_string(), toml::Value::String("polo".to_string())),
                ("SUN".to_string(), toml::Value::String("chips".to_string())),
            ])),
            reload_keybind: Keybind {
                modifiers: vec![Modifier::Ctrl, Modifier::Alt],
                key: Key::R,
            },
            kill_keybind: Keybind {
                modifiers: vec![Modifier::Ctrl, Modifier::Alt, Modifier::Shift],
                key: Key::Escape,
            },
            socket_dir: Some("/path/to/socket/dir".to_string()),
        };

        assert_eq!(
            parse_metaconfig(metaconfig_dir.path())?,
            expected_metaconfig
        );

        Ok(())
    }

    #[test]
    fn minimal_metaconfig_successfully_parses() -> anyhow::Result<()> {
        let metaconfig_text = r#"
            command = ["lua", "init.lua"]

            reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
            kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }
        "#;

        let metaconfig_dir = tempfile::tempdir()?;
        std::fs::write(
            metaconfig_dir.path().join("metaconfig.toml"),
            metaconfig_text,
        )?;

        let expected_metaconfig = Metaconfig {
            command: vec!["lua".to_string(), "init.lua".to_string()],
            envs: None,
            reload_keybind: Keybind {
                modifiers: vec![Modifier::Ctrl, Modifier::Alt],
                key: Key::R,
            },
            kill_keybind: Keybind {
                modifiers: vec![Modifier::Ctrl, Modifier::Alt, Modifier::Shift],
                key: Key::Escape,
            },
            socket_dir: None,
        };

        assert_eq!(
            parse_metaconfig(metaconfig_dir.path())?,
            expected_metaconfig
        );

        Ok(())
    }

    #[test]
    fn incorrect_metaconfig_does_not_parse() -> anyhow::Result<()> {
        let metaconfig_text = r#"
            command = "lua" # not an array

            reload_keybind = { modifiers = ["Ctrl", "Alt"], key = "r" }
            # Missing `kill_keybind`
            # kill_keybind = { modifiers = ["Ctrl", "Alt", "Shift"], key = "escape" }
        "#;

        let metaconfig_dir = tempfile::tempdir()?;
        std::fs::write(
            metaconfig_dir.path().join("metaconfig.toml"),
            metaconfig_text,
        )?;

        assert!(parse_metaconfig(metaconfig_dir.path()).is_err());

        Ok(())
    }

    // Ayo can we get Kotlin style test function naming so I can do something like
    // fn `config.dir with --config-dir returns correct dir`() {}
    #[test]
    fn config_dot_dir_with_dash_dash_config_dir_returns_correct_dir() -> anyhow::Result<()> {
        let dir = PathBuf::from("/some/dir/here");
        let config = Config::new(false, Some(dir.clone()));

        assert_eq!(config.dir(&BaseDirectories::with_prefix("pinnacle")?), dir);

        Ok(())
    }

    #[test]
    fn config_dot_dir_without_dash_dash_config_dir_returns_correct_dir() -> anyhow::Result<()> {
        let config = Config::new(false, None);
        let xdg_base_dirs = BaseDirectories::with_prefix("pinnacle")?;

        assert_eq!(config.dir(&xdg_base_dirs), get_config_dir(&xdg_base_dirs));

        Ok(())
    }
}
