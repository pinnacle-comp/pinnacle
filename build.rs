fn main() {
    println!("cargo:rerun-if-changed=api/lua_grpc");
    println!("cargo:rerun-if-changed=api/protocol");

    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let data_dir = xdg.create_data_directory("").unwrap();

    let remove_protos = format!("rm -r {data_dir:?}/protobuf");
    let copy_protos = format!("cp -r ./api/protocol {data_dir:?}/protobuf");

    let remove_default_config = format!("rm -r {data_dir:?}/default_config");
    let copy_default_config =
        format!("cp -r ./api/lua_grpc/examples/default {data_dir:?}/default_config");

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

    std::process::Command::new("/bin/sh")
        .arg("install_libs.sh")
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::env::set_current_dir("api/lua_grpc").unwrap();
    std::process::Command::new("luarocks")
        .arg("make")
        .arg("--local")
        .spawn()
        .expect("Luarocks is not installed");
}
