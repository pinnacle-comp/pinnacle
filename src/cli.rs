use std::{io::IsTerminal, path::PathBuf};

use clap::{Parser, ValueHint};
use tracing::{error, warn};

/// Valid backends that Pinnacle can run.
#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum Backend {
    /// Run Pinnacle in a window in your graphical environment
    Winit,
    /// Run Pinnacle from a tty
    Udev,
}

/// The main CLI struct.
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Start Pinnacle with the config at this directory
    #[arg(short, long, value_name("DIR"), value_hint(ValueHint::DirPath))]
    pub config_dir: Option<PathBuf>,

    /// Run Pinnacle with the specified backend
    ///
    /// This is usually not necessary, but if your environment variables are mucked up
    /// then this can be used to choose a backend.
    #[arg(short, long)]
    pub backend: Option<Backend>,

    /// Force Pinnacle to run with the provided backend
    #[arg(long, requires = "backend")]
    pub force: bool,

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

    /// Cli subcommands
    #[command(subcommand)]
    subcommand: Option<CliSubcommand>,
}

impl Cli {
    pub fn parse_and_prompt() -> Option<Self> {
        let mut cli = Cli::parse();

        // oh my god rustfmt is starting to piss me off

        cli.config_dir = cli.config_dir.and_then(|dir| {
            let new_dir = shellexpand::path::full(&dir);
            match new_dir {
                Ok(new_dir) => Some(new_dir.to_path_buf()),
                Err(err) => {
                    warn!("Could not shellexpand `--config-dir`'s argument: {err}; unsetting `--config-dir`");
                    None
                }
            }
        });

        if let Some(subcommand) = &cli.subcommand {
            match subcommand {
                CliSubcommand::Config(ConfigSubcommand::Gen(config_gen)) => {
                    if let Err(err) = generate_config(config_gen.clone()) {
                        error!("Error generating config: {err}");
                    }
                }
            }
            return None;
        }

        Some(cli)
    }
}

/// Cli subcommands.
#[derive(clap::Subcommand, Debug)]
enum CliSubcommand {
    /// Commands dealing with configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),
}

/// Config subcommands
#[derive(clap::Subcommand, Debug)]
enum ConfigSubcommand {
    /// Generate a config
    ///
    /// If not all flags are provided, this will launch an
    /// interactive prompt unless `--non-interactive` is passed
    /// or this is run in a non-interactive shell.
    Gen(ConfigGen),
}

/// Config arguments.
#[derive(clap::Args, Debug, Clone, PartialEq)]
struct ConfigGen {
    /// Generate a config in a specific language
    #[arg(short, long)]
    pub lang: Option<Lang>,

    /// Generate a config at this directory
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
enum Lang {
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
fn generate_config(args: ConfigGen) -> anyhow::Result<()> {
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

    let exit_message = |msg: &str| {
        if interactive {
            cliclack::outro_cancel(msg).expect("failed to display outro_cancel");
        } else {
            eprintln!("{msg}, exiting");
        }
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
                exit_message("Directory must be empty to create a Rust config in it");
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
            exit_message("Config generation cancelled.");
            return Ok(());
        }
    }

    std::fs::create_dir_all(&target_dir)?;

    // Generate the config

    let xdg_base_dirs = xdg::BaseDirectories::with_prefix("pinnacle")?;
    let mut default_config_dir = xdg_base_dirs.get_data_file("default_config");

    // %F = %Y-%m-%d or year-month-day in ISO 8601
    // %T = %H:%M:%S
    let now = format!("{}", chrono::Local::now().format("%F.%T"));

    match lang {
        Lang::Lua => {
            default_config_dir.push("lua");

            let mut files_to_backup: Vec<(String, String)> = Vec::new();

            for file in std::fs::read_dir(&default_config_dir)? {
                let file = file?;
                let name = file.file_name();
                let target_file = target_dir.join(&name);
                if let Ok(true) = target_file.try_exists() {
                    let backup_name = format!("{}.{now}.bak", name.to_string_lossy());
                    files_to_backup.push((name.to_string_lossy().to_string(), backup_name));
                }
            }

            if !files_to_backup.is_empty() {
                let msg = files_to_backup
                    .iter()
                    .map(|(src, dst)| format!("{src} -> {dst}"))
                    .collect::<Vec<_>>()
                    .join("\n");

                if interactive {
                    cliclack::note("The following files will be renamed:", msg)?;
                    let r#continue = cliclack::confirm("Continue?").interact()?;

                    if !r#continue {
                        exit_message("Config generation cancelled.");
                        return Ok(());
                    }
                } else {
                    println!("The following files will be renamed:");
                    println!("{msg}");
                }

                for (src, dst) in files_to_backup.iter() {
                    std::fs::rename(target_dir.join(src), target_dir.join(dst))?;
                }

                message("Renamed old files", Level::Info)?;
            }

            dircpy::copy_dir(&default_config_dir, &target_dir)?;
        }
        Lang::Rust => {
            default_config_dir.push("rust");

            assert!(
                std::fs::read_dir(&target_dir)?.next().is_none(),
                "target directory was not empty"
            );

            dircpy::copy_dir(&default_config_dir, &target_dir)?;
        }
    }

    message("Copied new config over", Level::Info)?;

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

#[cfg(test)]
mod tests {
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
            temp_dir
                .to_str()
                .ok_or(anyhow::anyhow!("not valid unicode"))?,
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

        assert!(matches!(
            temp_dir.join("default_config.lua").try_exists(),
            Ok(true)
        ));
        assert!(matches!(
            temp_dir.join("metaconfig.toml").try_exists(),
            Ok(true)
        ));
        assert!(matches!(
            temp_dir.join(".luarc.json").try_exists(),
            Ok(true)
        ));

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

        assert!(matches!(
            temp_dir.join("src/main.rs").try_exists(),
            Ok(true)
        ));
        assert!(matches!(
            temp_dir.join("metaconfig.toml").try_exists(),
            Ok(true)
        ));
        assert!(matches!(temp_dir.join("Cargo.toml").try_exists(), Ok(true)));

        Ok(())
    }

    #[test]
    fn non_interactive_config_gen_lua_backup_works() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let temp_dir = temp_dir
            .path()
            .join("non_interactive_config_gen_lua_backup_works");

        let config_gen = ConfigGen {
            lang: Some(Lang::Lua),
            dir: Some(temp_dir.clone()),
            non_interactive: true,
        };

        generate_config(config_gen.clone())?;
        generate_config(config_gen)?;

        let generated_file_count = std::fs::read_dir(&temp_dir)?
            .collect::<Result<Vec<_>, _>>()?
            .len();

        // 3 for original, 3 for backups
        assert_eq!(generated_file_count, 6);

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
