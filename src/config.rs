pub mod api;

use crate::{
    config::api::{msg::ModifierMask, PinnacleSocketSource},
    output::OutputName,
    tag::Tag,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::Context;
use smithay::{
    input::keyboard::keysyms,
    utils::{Logical, Point},
};
use toml::Table;

use api::msg::Modifier;

use crate::{
    state::{State, WithState},
    tag::TagId,
};

use self::api::msg::{
    window_rules::{WindowRule, WindowRuleCondition},
    CallbackId,
};

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
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

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

#[derive(Default, Debug)]
pub struct Config {
    pub window_rules: Vec<(WindowRuleCondition, WindowRule)>,
    pub output_callback_ids: Vec<CallbackId>,
    pub connector_saved_states: HashMap<OutputName, ConnectorSavedState>,
}

/// State saved when an output is disconnected. When the output is reconnected to the same
/// connector, the saved state will apply to restore its state.
#[derive(Debug, Default, Clone)]
pub struct ConnectorSavedState {
    pub loc: Point<i32, Logical>,
    pub tags: Vec<Tag>,
}

/// Parse a metaconfig file in `config_dir`, if any.
fn parse(config_dir: &Path) -> anyhow::Result<Metaconfig> {
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

        tracing::info!("Starting config");
        tracing::debug!("Clearing tags");

        for output in self.space.outputs() {
            output.with_state(|state| state.tags.clear());
        }

        TagId::reset();

        tracing::debug!("Clearing mouse and keybinds");
        self.input_state.keybinds.clear();
        self.input_state.mousebinds.clear();
        self.input_state.libinput_settings.clear();
        self.config.window_rules.clear();

        tracing::debug!("Killing old config");
        if let Some(channel) = self.api_state.kill_channel.as_ref() {
            if let Err(err) = futures_lite::future::block_on(channel.send(())) {
                tracing::warn!("failed to send kill ping to config future: {err}");
            }
        }

        if let Some(token) = self.api_state.socket_token {
            // Should only happen if parsing the metaconfig failed
            self.loop_handle.remove(token);
        }

        let tx_channel = self.api_state.tx_channel.clone();

        // Love that trailing slash
        let data_home = PathBuf::from(
            crate::XDG_BASE_DIRS
                .get_data_home()
                .to_string_lossy()
                .to_string()
                .trim_end_matches('/'),
        );
        std::env::set_var("PINNACLE_LIB_DIR", data_home);

        tracing::debug!("config dir is {:?}", config_dir);

        let metaconfig = match parse(config_dir) {
            Ok(metaconfig) => metaconfig,
            Err(_) => {
                self.start_config(crate::XDG_BASE_DIRS.get_data_home().join("lua"))?;
                return Ok(());
            }
        };

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
                .unwrap_or(PathBuf::from(crate::config::api::DEFAULT_SOCKET_DIR))
        };

        let socket_source = PinnacleSocketSource::new(tx_channel, &socket_dir)
            .context("Failed to create socket source")?;

        let reload_keybind = metaconfig.reload_keybind;
        let kill_keybind = metaconfig.kill_keybind;

        let mut command = metaconfig.command.iter();

        let arg1 = command
            .next()
            .context("command in metaconfig.toml was empty")?;

        let command = command.collect::<Vec<_>>();

        tracing::debug!(arg1, ?command);

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

        let mut child = async_process::Command::new(arg1)
            .args(command)
            .envs(envs)
            .current_dir(config_dir)
            .stdout(async_process::Stdio::inherit())
            .stderr(async_process::Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .context("failed to spawn config")?;

        tracing::info!("Started config with {:?}", metaconfig.command);

        let reload_mask = ModifierMask::from(reload_keybind.modifiers);
        let kill_mask = ModifierMask::from(kill_keybind.modifiers);

        let reload_keybind = (reload_mask, reload_keybind.key as u32);
        let kill_keybind = (kill_mask, kill_keybind.key as u32);

        let socket_token = self
            .loop_handle
            .insert_source(socket_source, |stream, _, data| {
                if let Some(old_stream) = data
                    .state
                    .api_state
                    .stream
                    .replace(Arc::new(Mutex::new(stream)))
                {
                    old_stream
                        .lock()
                        .expect("Couldn't lock old stream")
                        .shutdown(std::net::Shutdown::Both)
                        .expect("Couldn't shutdown old stream");
                }
            })?;

        self.input_state.reload_keybind = Some(reload_keybind);
        self.input_state.kill_keybind = Some(kill_keybind);
        self.api_state.socket_token = Some(socket_token);

        let (kill_channel, future_channel) = async_channel::unbounded::<()>();

        self.api_state.kill_channel = Some(kill_channel);
        self.api_state.future_channel = Some(future_channel.clone());

        let loop_handle = self.loop_handle.clone();

        enum Either {
            First,
            Second,
        }

        // We can't get at the child while it's in the executor, so in order to kill it we need a
        // channel that, when notified, will cause the child to be dropped and terminated.
        self.async_scheduler.schedule(async move {
            let which = futures_lite::future::race(
                async move {
                    let _ = child.status().await;
                    Either::First
                },
                async move {
                    let _ = future_channel.recv().await;
                    Either::Second
                },
            )
            .await;

            if let Either::First = which {
                tracing::warn!("Config crashed, loading default");

                loop_handle.insert_idle(|data| {
                    data.state
                        .start_config(crate::XDG_BASE_DIRS.get_data_home().join("lua"))
                        .expect("failed to load default config");
                });
            }
        })?;

        Ok(())
    }
}
