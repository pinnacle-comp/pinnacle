use std::path::{Path, PathBuf};

use toml::Table;

use crate::api::msg::Modifier;

#[derive(serde::Deserialize, Debug)]
pub struct Metaconfig {
    pub command: String,
    pub envs: Option<Table>,
    pub reload_keybind: Keybind,
    pub kill_keybind: Keybind,
    pub socket_dir: Option<PathBuf>,
}

#[derive(serde::Deserialize, Debug)]
pub struct Keybind {
    pub modifiers: Vec<Modifier>,
    pub key: Key,
}

#[derive(serde::Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    #[serde(rename = "1")]
    One,
    #[serde(rename = "2")]
    Two,
    #[serde(rename = "3")]
    Three,
    #[serde(rename = "4")]
    Four,
    #[serde(rename = "5")]
    Five,
    #[serde(rename = "6")]
    Six,
    #[serde(rename = "7")]
    Seven,
    #[serde(rename = "8")]
    Eight,
    #[serde(rename = "9")]
    Nine,
    #[serde(rename = "0")]
    Zero,
    #[serde(rename = "num1")]
    NumOne,
    #[serde(rename = "num2")]
    NumTwo,
    #[serde(rename = "num3")]
    NumThree,
    #[serde(rename = "num4")]
    NumFour,
    #[serde(rename = "num5")]
    NumFive,
    #[serde(rename = "num6")]
    NumSix,
    #[serde(rename = "num7")]
    NumSeven,
    #[serde(rename = "num8")]
    NumEight,
    #[serde(rename = "num9")]
    NumNine,
    #[serde(rename = "num0")]
    NumZero,
    #[serde(alias = "esc")]
    Escape,
}

pub fn parse(config_dir: &Path) -> Result<Metaconfig, Box<dyn std::error::Error>> {
    let config_dir = config_dir.join("metaconfig.toml");

    let metaconfig = std::fs::read_to_string(config_dir)?;

    Ok(toml::from_str(&metaconfig)?)
}
