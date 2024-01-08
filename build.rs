fn main() {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/protocol");

    std::process::Command::new("/bin/sh")
        .arg("install_libs.sh")
        .spawn()
        .unwrap();

    tonic_build::compile_protos("api/protocol/request.proto").unwrap();
}
