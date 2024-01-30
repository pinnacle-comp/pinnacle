fn main() {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/protocol");

    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let proto_dir = xdg.place_data_file("protobuf").unwrap();
    let default_config_dir = xdg.place_data_file("default_config").unwrap();

    let remove_protos = format!("rm -r {proto_dir:?}");
    let copy_protos = format!("cp -r ./api/protocol {proto_dir:?}");

    let remove_default_config = format!("rm -r {default_config_dir:?}");
    let copy_default_config = format!("cp -r ./api/lua/examples/default {default_config_dir:?}");

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_protos)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_protos)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_default_config)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_default_config)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::env::set_current_dir("api/lua").unwrap();
    std::process::Command::new("luarocks")
        .arg("make")
        .arg("--local")
        .spawn()
        .expect("Luarocks is not installed")
        .wait()
        .unwrap();
}
