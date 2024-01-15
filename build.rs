fn main() {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/lua_grpc");
    println!("cargo:rerun-if-changed=api/protocol");

    let xdg = xdg::BaseDirectories::with_prefix("pinnacle").unwrap();

    let data_dir = xdg.create_data_directory("").unwrap();

    let remove_protos = format!("rm -r {data_dir:?}/protobuf");
    let remove_lua = format!("rm -r {data_dir:?}/lua_grpc");
    let copy_protos = format!("cp -r ./api/protocol {data_dir:?}/protobuf");
    let copy_lua = format!("cp -r ./api/lua_grpc {data_dir:?}");

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_protos)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(&remove_lua)
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
        .arg(&copy_lua)
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
}
