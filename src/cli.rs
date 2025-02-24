use std::{io::IsTerminal, path::PathBuf};

use clap::{Parser, ValueHint};
use tracing::warn;

/// Valid backends that Pinnacle can run.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Backend {
    /// Run Pinnacle in a window in your graphical environment
    Winit,
    /// Run Pinnacle from a tty
    Udev,
    /// Run the dummy backend
    ///
    /// This does not open a window and is used only for testing.
    #[cfg(feature = "testing")]
    Dummy,
}

/// The main CLI struct.
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None, args_conflicts_with_subcommands = true)]
pub struct Cli {
    /// Use the config at the given directory
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    pub config_dir: Option<PathBuf>,

    /// Allow running Pinnacle as root (this is NOT recommended)
    #[arg(long)]
    pub allow_root: bool,

    /// Start Pinnacle without a config
    ///
    /// This is meant to be used for debugging.
    /// Additionally, Pinnacle will not load the
    /// default config if a manually spawned one
    /// crashes or exits.
    #[arg(long)]
    pub no_config: bool,

    /// Prevent Xwayland from being started
    #[arg(long)]
    pub no_xwayland: bool,

    /// Open the gRPC socket at the specified directory
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    pub socket_dir: Option<PathBuf>,

    /// Start Pinnacle as a session
    ///
    /// This will import the environment into systemd and D-Bus.
    #[arg(long)]
    pub session: bool,

    /// Cli subcommands
    #[command(subcommand)]
    pub subcommand: Option<CliSubcommand>,
}

impl Cli {
    pub fn parse() -> Self {
        let mut cli: Self = Parser::parse();

        cli.config_dir = cli.config_dir.and_then(|dir| {
            let new_dir = shellexpand::path::full(&dir);
            match new_dir {
                Ok(new_dir) => Some(new_dir.to_path_buf()),
                Err(err) => {
                    warn!("Could not expand home in `--config-dir`: {err}; unsetting");
                    None
                }
            }
        });

        cli
    }
}

/// Cli subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum CliSubcommand {
    /// Commands dealing with configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),

    /// Commands for debugging
    #[command(subcommand)]
    Debug(DebugSubcommand),

    /// Print build and system information
    Info,

    /// Generate shell completions and print them to stdout
    GenCompletions {
        #[arg(short, long)]
        shell: clap_complete::Shell,
    },
}

/// Config subcommands
#[derive(clap::Subcommand, Debug)]
pub enum ConfigSubcommand {
    /// Generate a config
    ///
    /// If not all flags are provided, this will launch an
    /// interactive prompt unless `--non-interactive` is passed
    /// or this is run in a non-interactive shell.
    Gen(ConfigGen),
}

/// Config arguments.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct ConfigGen {
    /// Generate a config in a specific language
    #[arg(short, long)]
    pub lang: Option<Lang>,

    /// Generate a config at the given directory
    #[arg(short, long, value_hint(ValueHint::DirPath))]
    pub dir: Option<PathBuf>,

    /// Do not show interactive prompts if both `--lang` and `--dir` are set
    ///
    /// This does nothing inside of non-interactive shells.
    #[arg(
        short,
        long,
        requires("lang"),
        requires("dir"),
        default_value_t = !std::io::stdout().is_terminal()
    )]
    pub non_interactive: bool,
}

/// Possible languages for configuration.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// Generate a Lua config
    Lua,
    /// Generate a Rust config
    Rust,
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

//////////////////////////////////////////////////////////////////////

/// Generate a new config.
///
/// If `--non-interactive` is passed or the shell is non-interactive, this will not
/// output interactive prompts.
pub fn generate_config(args: ConfigGen) -> anyhow::Result<()> {
    let interactive = !args.non_interactive;

    if !interactive && (args.lang.is_none() || args.dir.is_none()) {
        eprintln!("Error: both `--lang` and `--dir` must be set in a non-interactive shell.");
        return Ok(());
    }

    if interactive {
        cliclack::intro("Welcome to the interactive config generator!")?;
        tokio::spawn(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to listen for ctrl-c");
        });
    }

    enum Level {
        Info,
        Success,
    }

    let message = |msg: &str, level: Level| -> anyhow::Result<()> {
        if interactive {
            Ok(match level {
                Level::Info => cliclack::log::info(msg),
                Level::Success => cliclack::log::success(msg),
            }?)
        } else {
            println!("{msg}");
            Ok(())
        }
    };

    let exit_message = |msg: &str| -> anyhow::Result<()> {
        if interactive {
            cliclack::outro_cancel(msg)?;
        } else {
            eprintln!("{msg}, exiting");
        }

        Ok(())
    };

    let lang = match args.lang {
        Some(lang) => {
            let msg = format!("Select a language:\n{lang} (from -l/--lang)");
            message(&msg, Level::Success)?;

            lang
        }
        None => {
            assert!(interactive);

            cliclack::select("Select a language:")
                .items(&[(Lang::Lua, "Lua", ""), (Lang::Rust, "Rust", "")])
                .interact()?
        }
    };

    let default_dir = xdg::BaseDirectories::with_prefix("pinnacle")?.get_config_home();

    let default_dir_clone = default_dir.clone();

    let dir_validator = move |s: &String| {
        let mut target_dir = if s.is_empty() {
            default_dir_clone.clone()
        } else {
            PathBuf::from(
                shellexpand::full(s)
                    .map_err(|err| format!("Directory expansion failed: {err}"))?
                    .to_string(),
            )
        };

        if target_dir.is_relative() {
            let mut new_dir = std::env::current_dir().map_err(|err| {
                format!("Failed to get the current dir to resolve relative path: {err}")
            })?;
            new_dir.push(target_dir);
            target_dir = new_dir;
        }

        match target_dir.try_exists() {
            Ok(exists) => {
                if exists {
                    if !target_dir.is_dir() {
                        Err(format!(
                            "`{}` exists but is not a directory",
                            target_dir.display()
                        ))
                    } else if lang == Lang::Rust
                        && std::fs::read_dir(&target_dir)
                            .map_err(|err| {
                                format!(
                                    "Failed to check if `{}` is empty: {err}",
                                    target_dir.display()
                                )
                            })?
                            .next()
                            .is_some()
                    {
                        Err(format!(
                            "`{}` exists but is not empty. Empty it to generate a Rust config in it.",
                            target_dir.display()
                        ))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            }
            Err(err) => Err(format!(
                "Failed to check if `{}` exists: {err}",
                target_dir.display()
            )),
        }
    };

    let target_dir = match args.dir {
        Some(dir) => {
            let msg = format!(
                "Choose a directory to place the config in:\n{} (from -d/--dir)",
                dir.display()
            );

            message(&msg, Level::Success)?;

            if lang == Lang::Rust && matches!(dir.try_exists(), Ok(true)) {
                exit_message("Directory must be empty to create a Rust config in it")?;
                anyhow::bail!("{msg}");
            }

            dir
        }
        None => {
            assert!(interactive);

            let dir: PathBuf = cliclack::input("Choose a directory to place the config in:")
                .default_input(default_dir.to_string_lossy().as_ref())
                .validate_interactively(dir_validator)
                .interact()?;

            let mut dir = shellexpand::path::full(&dir)?.to_path_buf();

            if dir.is_relative() {
                let mut new_dir = std::env::current_dir()?;
                new_dir.push(dir);
                dir = new_dir;
            }

            dir
        }
    };

    if let Ok(false) = target_dir.try_exists() {
        let msg = format!(
            "`{}` doesn't exist and will be created.",
            target_dir.display()
        );
        message(&msg, Level::Info)?;
    }

    if interactive {
        let confirm_creation = cliclack::confirm(format!(
            "Create a {} config inside `{}`?",
            lang,
            target_dir.display()
        ))
        .initial_value(false)
        .interact()?;

        if !confirm_creation {
            exit_message("Config generation cancelled.")?;
            return Ok(());
        }
    }

    std::fs::create_dir_all(&target_dir)?;

    // Generate the config

    let res = crate::config::generate_config(
        &target_dir,
        match lang {
            Lang::Lua => crate::config::Lang::Lua,
            Lang::Rust => crate::config::Lang::Rust,
        },
    );

    if let Err(err) = res {
        let msg = format!("Error creating config: {err}");
        exit_message(&msg)?;
        anyhow::bail!("{err}");
    }

    let mut outro_msg = format!("{lang} config created in {}!", target_dir.display());
    if lang == Lang::Rust {
        outro_msg = format!(
            "{outro_msg}\nYou may want to run `cargo build` in the \
            config directory beforehand to avoid waiting when starting up Pinnacle."
        );
    }

    if interactive {
        cliclack::outro(outro_msg)?;
    } else {
        println!("{outro_msg}");
    }

    Ok(())
}

/// Config subcommands
#[derive(clap::Subcommand, Debug)]
pub enum DebugSubcommand {
    // Panic to check backtraces
    Panic,
}

#[cfg(test)]
mod tests {
    use anyhow::Context;
    use assert_matches::assert_matches;

    use super::*;

    // TODO: find a way to test the interactive bits programmatically

    #[test]
    fn cli_config_gen_parses_correctly() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir.path().join("cli_config_gen_parses_correctly");

        let cli = Cli::parse_from([
            "pinnacle",
            "config",
            "gen",
            "--lang",
            "rust",
            "--dir",
            temp_dir.to_str().context("not valid unicode")?,
            "--non-interactive",
        ]);

        let expected_config_gen = ConfigGen {
            lang: Some(Lang::Rust),
            dir: Some(temp_dir.to_path_buf()),
            non_interactive: true,
        };

        let Some(CliSubcommand::Config(ConfigSubcommand::Gen(config_gen))) = cli.subcommand else {
            anyhow::bail!("cli.subcommand config_gen doesn't exist");
        };

        assert_eq!(config_gen, expected_config_gen);

        let cli = Cli::parse_from(["pinnacle", "config", "gen", "--lang", "lua"]);

        let expected_config_gen = ConfigGen {
            lang: Some(Lang::Lua),
            dir: None,
            non_interactive: !std::io::stdout().is_terminal(),
        };

        let Some(CliSubcommand::Config(ConfigSubcommand::Gen(config_gen))) = cli.subcommand else {
            anyhow::bail!("cli.subcommand config_gen doesn't exist");
        };

        assert_eq!(config_gen, expected_config_gen);

        Ok(())
    }

    #[test]
    fn non_interactive_config_gen_lua_works() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir.path().join("non_interactive_config_gen_lua_works");

        let config_gen = ConfigGen {
            lang: Some(Lang::Lua),
            dir: Some(temp_dir.clone()),
            non_interactive: true,
        };

        generate_config(config_gen)?;

        assert_matches!(temp_dir.join("default_config.lua").try_exists(), Ok(true));
        assert_matches!(temp_dir.join("pinnacle.toml").try_exists(), Ok(true));
        assert_matches!(temp_dir.join(".luarc.json").try_exists(), Ok(true));

        Ok(())
    }

    #[test]
    fn non_interactive_config_gen_rust_works() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir
            .path()
            .join("non_interactive_config_gen_rust_works");

        let config_gen = ConfigGen {
            lang: Some(Lang::Rust),
            dir: Some(temp_dir.clone()),
            non_interactive: true,
        };

        generate_config(config_gen)?;

        assert_matches!(temp_dir.join("src/main.rs").try_exists(), Ok(true));
        assert_matches!(temp_dir.join("pinnacle.toml").try_exists(), Ok(true));
        assert_matches!(temp_dir.join("Cargo.toml").try_exists(), Ok(true));

        Ok(())
    }

    #[test]
    fn non_interactive_config_gen_rust_nonempty_dir_does_not_work() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir
            .path()
            .join("non_interactive_config_gen_rust_nonempty_dir_does_not_work");

        let config_gen = ConfigGen {
            lang: Some(Lang::Rust),
            dir: Some(temp_dir),
            non_interactive: true,
        };

        generate_config(config_gen.clone())?;

        assert!(generate_config(config_gen.clone()).is_err());

        Ok(())
    }
}
