use crate::{
    api::{
        InputService, OutputService, PinnacleService, ProcessService, TagService, WindowService,
    },
    input::ModifierMask,
    output::OutputName,
    state::CalloopData,
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
    output::v0alpha1::{output_service_server::OutputServiceServer, ConnectForAllResponse},
    process::v0alpha1::process_service_server::ProcessServiceServer,
    tag::v0alpha1::tag_service_server::TagServiceServer,
    v0alpha1::pinnacle_service_server::PinnacleServiceServer,
    window::v0alpha1::window_service_server::WindowServiceServer,
};
use smithay::{
    input::keyboard::keysyms,
    reexports::calloop::{self, channel::Event, LoopHandle, RegistrationToken},
    utils::{Logical, Point},
};
use sysinfo::ProcessRefreshKind;
use tokio::{sync::mpsc::UnboundedSender, task::JoinHandle};
use toml::Table;

use xkbcommon::xkb::Keysym;

use crate::{
    state::{State, WithState},
    tag::TagId,
};

const DEFAULT_SOCKET_DIR: &str = "/tmp";

/// The metaconfig struct containing what to run, what envs to run it with, various keybinds, and
/// the target socket directory.
#[derive(serde::Deserialize, Debug)]
pub struct Metaconfig {
    pub command: Vec<String>,
    pub envs: Option<Table>,
    pub reload_keybind: Keybind,
    pub kill_keybind: Keybind,
    pub socket_dir: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Keybind {
    modifiers: Vec<Modifier>,
    key: Key,
}

#[derive(serde::Deserialize, Debug, Clone, Copy)]
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
#[derive(serde::Deserialize, Debug, Clone, Copy)]
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
    pub output_callback_senders: Vec<UnboundedSender<Result<ConnectForAllResponse, tonic::Status>>>,
    /// Saved states when outputs are disconnected
    pub connector_saved_states: HashMap<OutputName, ConnectorSavedState>,

    config_join_handle: Option<JoinHandle<()>>,
    config_reload_on_crash_token: Option<RegistrationToken>,
}

impl Config {
    pub fn clear(&mut self, loop_handle: &LoopHandle<CalloopData>) {
        self.window_rules.clear();
        self.output_callback_senders.clear();
        self.connector_saved_states.clear();
        if let Some(join_handle) = self.config_join_handle.take() {
            join_handle.abort();
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
}

/// Parse a metaconfig file in `config_dir`, if any.
fn parse_metaconfig(config_dir: &Path) -> anyhow::Result<Metaconfig> {
    let config_dir = config_dir.join("metaconfig.toml");

    let metaconfig =
        std::fs::read_to_string(config_dir).context("Failed to read metaconfig.toml")?;

    toml::from_str(&metaconfig).context("Failed to deserialize toml")
}

/// Get the config dir. This is $PINNACLE_CONFIG_DIR, then $XDG_CONFIG_HOME/pinnacle,
/// then ~/.config/pinnacle.
pub fn get_config_dir() -> PathBuf {
    let config_dir = std::env::var("PINNACLE_CONFIG_DIR")
        .ok()
        .and_then(|s| Some(PathBuf::from(shellexpand::full(&s).ok()?.to_string())));

    config_dir.unwrap_or(crate::XDG_BASE_DIRS.get_config_home())
}

impl State {
    /// Start the config in `config_dir`.
    ///
    /// If this method is called while a config is already running, it will be replaced.
    pub fn start_config(&mut self, config_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let config_dir = config_dir.as_ref();

        tracing::info!("Starting config at {}", config_dir.display());

        let default_lua_config_dir = crate::XDG_BASE_DIRS.get_data_file("default_config");

        let load_default_config = |state: &mut State, reason: &str| {
            tracing::error!(
                "Unable to load config at {}: {reason}",
                config_dir.display()
            );
            tracing::info!("Falling back to default Lua config");
            state.start_config(&default_lua_config_dir)
        };

        let metaconfig = match parse_metaconfig(config_dir) {
            Ok(metaconfig) => metaconfig,
            Err(err) => {
                // Stops infinite recursion if somehow the default_config dir is screwed up
                if config_dir == default_lua_config_dir {
                    tracing::error!("The metaconfig at the default Lua config directory is either malformed or missing.");
                    tracing::error!(
                        "If you have not touched {}, this is a bug and you should file an issue (pretty please with a cherry on top?).",
                        default_lua_config_dir.display()
                    );
                    anyhow::bail!("default lua config dir does not work");
                }
                return load_default_config(self, &err.to_string());
            }
        };

        tracing::debug!("Clearing tags");
        for output in self.space.outputs() {
            output.with_state(|state| state.tags.clear());
        }

        TagId::reset();

        tracing::debug!("Clearing input state");

        self.input_state.clear();

        self.config.clear(&self.loop_handle);

        // Because the grpc server is implemented to only start once,
        // any updates to `socket_dir` won't be applied until restart.
        if self.grpc_server_join_handle.is_none() {
            // If a socket is provided in the metaconfig, use it.
            let socket_dir = if let Some(socket_dir) = &metaconfig.socket_dir {
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
                crate::XDG_BASE_DIRS
                    .get_runtime_directory()
                    .cloned()
                    .unwrap_or(PathBuf::from(DEFAULT_SOCKET_DIR))
            };

            self.start_grpc_server(socket_dir.as_path())?;
        }

        let reload_keybind = metaconfig.reload_keybind;
        let kill_keybind = metaconfig.kill_keybind;

        let mut command = metaconfig.command.iter();

        let arg0 = match command.next() {
            Some(arg0) => arg0,
            None => return load_default_config(self, "no command specified"),
        };

        let command = command.collect::<Vec<_>>();

        tracing::debug!(arg0, ?command);

        let envs = metaconfig
            .envs
            .unwrap_or(toml::map::Map::new())
            .into_iter()
            .filter_map(|(key, val)| {
                if let toml::Value::String(string) = val {
                    Some((
                        key,
                        shellexpand::full_with_context(
                            &string,
                            || std::env::var("HOME").ok(),
                            // Expand nonexistent vars to an empty string instead of crashing
                            |var| Ok::<_, ()>(Some(std::env::var(var).unwrap_or("".to_string()))),
                        )
                        .ok()?
                        .to_string(),
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        tracing::debug!("Config envs are {envs:?}");

        let mut child = match tokio::process::Command::new(arg0)
            .args(command)
            .envs(envs)
            .current_dir(config_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(err) => return load_default_config(self, &err.to_string()),
        };

        tracing::info!("Started config with {:?}", metaconfig.command);

        let reload_mask = ModifierMask::from(reload_keybind.modifiers);
        let kill_mask = ModifierMask::from(kill_keybind.modifiers);

        let reload_keybind = (reload_mask, Keysym::from(reload_keybind.key as u32));
        let kill_keybind = (kill_mask, Keysym::from(kill_keybind.key as u32));

        self.input_state.reload_keybind = Some(reload_keybind);
        self.input_state.kill_keybind = Some(kill_keybind);

        let (pinger, ping_source) = calloop::ping::make_ping()?;

        let token = self
            .loop_handle
            .insert_source(ping_source, move |_, _, data| {
                tracing::error!("Config crashed! Falling back to default Lua config");
                data.state
                    .start_config(&default_lua_config_dir)
                    .expect("failed to start default lua config");
            })?;

        self.config.config_join_handle = Some(tokio::spawn(async move {
            let _ = child.wait().await;
            pinger.ping();
        }));

        self.config.config_reload_on_crash_token = Some(token);

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
                tracing::debug!("Removing socket at {:?}", file.path());
                std::fs::remove_file(file.path())
                    .context(format!("Failed to remove old socket at {:?}", file.path()))?;
            }
        }

        std::env::set_var(
            "PINNACLE_PROTO_DIR",
            crate::XDG_BASE_DIRS.get_data_file("protobuf"),
        );

        let (grpc_sender, grpc_receiver) =
            calloop::channel::channel::<Box<dyn FnOnce(&mut Self) + Send>>();

        self.loop_handle
            .insert_source(grpc_receiver, |msg, _, data| match msg {
                Event::Msg(f) => f(&mut data.state),
                Event::Closed => tracing::error!("grpc receiver was closed"),
            })
            .expect("failed to insert grpc_receiver into loop");

        let pinnacle_service = PinnacleService {
            sender: grpc_sender.clone(),
        };
        let input_service = InputService {
            sender: grpc_sender.clone(),
        };
        let process_service = ProcessService {
            sender: grpc_sender.clone(),
        };
        let tag_service = TagService {
            sender: grpc_sender.clone(),
        };
        let output_service = OutputService {
            sender: grpc_sender.clone(),
        };
        let window_service = WindowService {
            sender: grpc_sender.clone(),
        };

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
            .add_service(WindowServiceServer::new(window_service));

        match self.xdisplay.as_ref() {
            Some(_) => {
                self.grpc_server_join_handle = Some(tokio::spawn(async move {
                    if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                        tracing::error!("gRPC server error: {err}");
                    }
                }));
            }
            // FIXME: Not really high priority but if you somehow reload the config really, REALLY
            // |      fast at startup then I think there's a chance that the gRPC server
            // |      could get started twice.
            None => self.schedule(
                |data| data.state.xdisplay.is_some(),
                move |data| {
                    data.state.grpc_server_join_handle = Some(tokio::spawn(async move {
                        if let Err(err) = grpc_server.serve_with_incoming(uds_stream).await {
                            tracing::error!("gRPC server error: {err}");
                        }
                    }));
                },
            ),
        }

        Ok(())
    }
}
