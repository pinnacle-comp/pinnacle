use std::{path::Path, process::Command};

use clap::{Parser, Subcommand};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))?;

    let cli = Cli::parse();

    match cli.subcmd {
        Subcmd::Install => install()?,
        Subcmd::Build { args } => build(args)?,
        Subcmd::Run { args } => run(args)?,
    }

    Ok(())
}

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcmd: Subcmd,
}

#[derive(Subcommand, Clone)]
enum Subcmd {
    /// Install the default configs and Lua library (if Luarocks is detected)
    Install,
    /// Runs `install` and builds the project
    ///
    /// Additionally pre-builds the default Rust config.
    Build {
        #[arg(trailing_var_arg(true), allow_hyphen_values(true))]
        args: Option<Vec<String>>,
    },
    /// Runs `install` and runs the project
    Run {
        #[arg(trailing_var_arg(true), allow_hyphen_values(true))]
        args: Option<Vec<String>>,
    },
}

fn install() -> Result<(), Box<dyn std::error::Error>> {
    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let proto_dir = xdg.place_data_file("protobuf").unwrap();
    let default_config_dir = xdg.place_data_file("default_config").unwrap();
    let default_lua_config_dir = default_config_dir.join("lua");
    let default_rust_config_dir = default_config_dir.join("rust");

    std::fs::remove_dir_all(&proto_dir)?;
    copy_dir("./api/protocol", &proto_dir)?;

    std::fs::remove_dir_all(&default_config_dir)?;
    std::fs::create_dir_all(&default_config_dir)?;
    copy_dir("./api/lua/examples/default", default_lua_config_dir)?;
    // Need to resolve symlinks so we use cp
    Command::new("cp")
        .args([
            "-LR".as_ref(),
            "./api/rust/examples/default_config/for_copying".as_ref(),
            default_rust_config_dir.as_os_str(),
        ])
        .spawn()?
        .wait()?;

    std::env::set_current_dir("api/lua").unwrap();
    match Command::new("luarocks").arg("make").arg("--local").spawn() {
        Ok(mut child) => {
            child.wait()?;
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                println!("Luarocks not found; skipping Lua library installation");
            } else {
                return Err(err.into());
            }
        }
    }

    Ok(())
}

fn build(build_args: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
    install()?;

    let build_args = build_args.unwrap_or_else(|| vec!["--package".into(), "pinnacle".into()]);
    println!("{} build {}", env!("CARGO"), build_args.join(" "));
    Command::new(env!("CARGO"))
        .arg("build")
        .args(build_args)
        .spawn()?
        .wait()?;

    Ok(())
}

fn run(run_args: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
    install()?;

    let run_args = run_args.unwrap_or_else(|| vec!["--package".into(), "pinnacle".into()]);
    println!("{} run {}", env!("CARGO"), run_args.join(" "));
    Command::new(env!("CARGO"))
        .arg("run")
        .args(run_args)
        .spawn()?
        .wait()?;

    Ok(())
}

fn copy_dir<P, Q>(from: P, to: Q) -> Result<(), std::io::Error>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    dircpy::copy_dir_advanced(from.as_ref(), to.as_ref(), true, true, true, vec![], vec![])
}
