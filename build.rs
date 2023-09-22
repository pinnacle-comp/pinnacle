fn main() {
    println!("cargo:rerun-if-changed=api/lua");

    std::process::Command::new("/bin/sh")
        .arg("install_libs.sh")
        .spawn()
        .unwrap();
}
