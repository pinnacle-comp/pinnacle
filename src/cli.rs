use std::{
    cell::RefCell,
    ffi::OsString,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
};

use clap::{error::ErrorKind, CommandFactory, Parser, ValueHint};
use cliclack::Validate;

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
    pub subcommand: Option<CliSubcommand>,
}

impl Cli {
    pub fn parse_and_prompt() -> Self {
        let args = Cli::parse();

        match &args.subcommand {
            Some(CliSubcommand::Config(ConfigSubcommand::Gen(config_gen))) => {
                generate_config(config_gen.clone()).unwrap();
            }
            None => (),
        }

        args
    }
}

/// Cli subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum CliSubcommand {
    /// Commands dealing with configuration
    #[command(subcommand)]
    Config(ConfigSubcommand),
}

/// Config subcommands
#[derive(clap::Subcommand, Debug)]
pub enum ConfigSubcommand {
    /// Generate a config
    ///
    /// If not all flags are provided, this will launch an
    /// interactive prompt.
    Gen(ConfigGen),
}

/// Config arguments.
#[derive(clap::Args, Debug, Clone)]
pub struct ConfigGen {
    /// Generate a config in a specific language
    #[arg(short, long)]
    pub lang: Option<Lang>,
    /// Generate a config at this directory
    #[arg(short, long, value_hint(ValueHint::DirPath))]
    pub dir: Option<PathBuf>,
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

/// Show the interactive prompt for config generation.
pub fn generate_config(args: ConfigGen) -> anyhow::Result<()> {
    cliclack::intro("Config generation")?;

    let mut skip_confirmation = true;

    let lang = match args.lang {
        Some(lang) => {
            cliclack::log::success(format!("Select a language:\n{lang} (from -l/--lang)"))?;
            lang
        }
        None => {
            skip_confirmation = false;
            cliclack::select("Select a language:")
                .items(&[(Lang::Lua, "Lua", ""), (Lang::Rust, "Rust", "")])
                .interact()?
        }
    };

    let dir = match args.dir {
        Some(dir) => {
            cliclack::log::success(format!(
                "Choose a directory to place the config in:\n{} (from -d/--dir)",
                dir.display()
            ))?;
            dir
        }
        None => {
            skip_confirmation = false;
            let mut wants_to_create_dir: Option<PathBuf> = None;
            let mut wants_to_create = false;

            let dir: String = cliclack::input("Choose a directory to place the config in:")
                // Now this is a grade A bastardization of what this function is supposed to do
                .validate_interactively(DirValidator::new(move |s: &String| {
                    let dir = shellexpand::full(s)
                        .map_err(|err| format!("Directory expansion failed: {err}"))?;
                    let mut dir = PathBuf::from(dir.to_string());

                    if dir.is_relative() {
                        let mut new_dir = std::env::current_dir().map_err(|err| {
                            format!("Failed to get the current dir to resolve relative path: {err}")
                        })?;
                        new_dir.push(dir);
                        dir = new_dir;
                    }

                    match dir.try_exists() {
                        Ok(exists) => {
                            if exists {
                                if !dir.is_dir() {
                                    Err(format!(
                                        "`{}` exists but is not a directory",
                                        dir.display()
                                    ))
                                } else {
                                    wants_to_create_dir = None;
                                    Ok(())
                                }
                            } else if wants_to_create_dir.as_ref() == Some(&dir) {
                                if wants_to_create {
                                    Ok(())
                                } else {
                                    wants_to_create = true;
                                    Err(format!(
                                        "`{}` doesn't exist. Press ENTER again to create it.",
                                        dir.display()
                                    ))
                                }
                            } else {
                                wants_to_create = false;
                                wants_to_create_dir = Some(dir.clone());
                                Err(format!(
                                    "`{}` doesn't exist. Press ENTER twice to create it.",
                                    dir.display()
                                ))
                            }
                        }
                        Err(err) => Err(format!(
                            "Failed to check if `{}` exists: {err}",
                            dir.display()
                        )),
                    }
                }))
                .interact()?;

            let dir = shellexpand::full(&dir)?;
            let mut dir = PathBuf::from(dir.to_string());

            if dir.is_relative() {
                let mut new_dir = std::env::current_dir()?;
                new_dir.push(dir);
                dir = new_dir;
            }

            dir
        }
    };

    if skip_confirmation {
        cliclack::log::info("Final confirmation: skipping because all flags were present")?;
    } else {
        let confirm_creation = cliclack::confirm(format!(
            "Final confirmation: create a {} config inside `{}`?",
            lang,
            dir.display()
        ))
        .initial_value(false)
        .interact()?;

        if !confirm_creation {
            cliclack::outro_cancel("Config generation cancelled.")?;
            anyhow::bail!("cancelled");
        } else {
            cliclack::log::info("HERE")?;
        }
    }

    // Generate the config

    let xdg_base_dirs = xdg::BaseDirectories::with_prefix("pinnacle")?;
    let mut default_config_dir = xdg_base_dirs.get_data_file("default_config");
    match lang {
        Lang::Lua => {
            cliclack::log::info("HERE 2")?;
            default_config_dir.push("lua");
            // %F = %Y-%m-%d or year-month-day in ISO 8601
            let now = format!("{}", chrono::Local::now().format("%F.%T"));
            let mut backed_up_files: Vec<(String, String)> = Vec::new();
            for file in std::fs::read_dir(&default_config_dir)? {
                let file = file?;
                let name = file.file_name();
                let target_file = dir.join(&name);
                if let Ok(true) = target_file.try_exists() {
                    let backup_name = format!("{}.{now}.bak", name.to_string_lossy());
                    backed_up_files.push((name.to_string_lossy().to_string(), backup_name));
                }
            }
            cliclack::log::info("HERE 3")?;

            if !backed_up_files.is_empty() {
                cliclack::log::info("HERE 4")?;
                let prompt = backed_up_files
                    .iter()
                    .map(|(src, dst)| format!("{src} -> {dst}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                cliclack::note("The following files will be renamed:", prompt)?;
                let r#continue = cliclack::confirm("Continue?").interact()?;

                if !r#continue {
                    cliclack::outro_cancel("Config generation cancelled.")?;
                    anyhow::bail!("cancelled");
                }

                for (src, dst) in backed_up_files.iter() {
                    std::fs::rename(dir.join(src), dir.join(dst))?;
                }

                cliclack::log::info("Renamed old files")?;

                dircpy::copy_dir(default_config_dir, dir)?;

                cliclack::log::info("Copied new config over")?;
            }
            cliclack::log::info("HERE END")?;
        }
        Lang::Rust => {
            default_config_dir.push("rust");
        }
    }

    cliclack::outro("Done!")?;

    Ok(())
}

struct DirValidator<T, F: FnMut(&T) -> Result<(), E>, E>(Rc<RefCell<F>>, PhantomData<(T, E)>);

impl<T, F, E> DirValidator<T, F, E>
where
    F: FnMut(&T) -> Result<(), E>,
{
    fn new(validator: F) -> Self {
        Self(Rc::new(RefCell::new(validator)), PhantomData)
    }
}

impl<T, F, E> Validate<T> for DirValidator<T, F, E>
where
    F: FnMut(&T) -> Result<(), E>,
{
    type Err = E;

    fn validate(&self, input: &T) -> Result<(), Self::Err> {
        let mut validator = self.0.borrow_mut();
        validator(input)
    }
}
