fn main() {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/lua_grpc");
    println!("cargo:rerun-if-changed=api/protocol");

    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let data_dir = xdg.create_data_directory("").unwrap();

    let copy_protos = format!("cp -r ./api/protocol {data_dir:?}/protobuf");
    let copy_lua = format!("cp -r ./api/lua_grpc {data_dir:?}");

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_protos)
        .spawn()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&copy_lua)
        .spawn()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("install_libs.sh")
        .spawn()
        .unwrap();
}
