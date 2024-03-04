use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/protocol");

    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let proto_dir = xdg.place_data_file("protobuf").unwrap();
    let default_config_dir = xdg.place_data_file("default_config").unwrap();
    let default_lua_config_dir = default_config_dir.join("lua");
    let default_rust_config_dir = default_config_dir.join("rust");

    let remove_protos = format!("rm -r {proto_dir:?}");
    let copy_protos = format!("cp -r ./api/protocol {proto_dir:?}");

    let remove_default_config_dir = format!("rm -r {default_config_dir:?}");

    let copy_default_lua_config =
        format!("cp -r ./api/lua/examples/default {default_lua_config_dir:?}");

    let copy_default_rust_config = format!(
        "cp -LR ./api/rust/examples/default_config/for_copying {default_rust_config_dir:?}"
    );

    Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_protos)
        .spawn()?
        .wait()?;

    Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_protos)
        .spawn()?
        .wait()?;

    Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_default_config_dir)
        .spawn()?
        .wait()?;

    std::fs::create_dir_all(&default_config_dir)?;

    Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_default_lua_config)
        .spawn()?
        .wait()?;

    Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_default_rust_config)
        .spawn()?
        .wait()?;

    std::env::set_current_dir("api/lua").unwrap();
    Command::new("luarocks")
        .arg("make")
        .arg("--local")
        .spawn()
        .expect("Luarocks is not installed")
        .wait()?;

    Ok(())
}
