use const_format::formatcp;

fn main() {
    println!("cargo:rerun-if-changed=api/lua");
    println!("cargo:rerun-if-changed=api/protocol");

    std::process::Command::new("/bin/sh")
        .arg("install_libs.sh")
        .spawn()
        .unwrap();

    tonic_build::compile_protos("api/protocol/request.proto").unwrap();

    const VERSION: &str = "v0alpha1";
    const PROTOS: &[&str] = &[
        formatcp!("api/protocol/pinnacle/{VERSION}/pinnacle.proto"),
        formatcp!("api/protocol/pinnacle/input/{VERSION}/input.proto"),
        formatcp!("api/protocol/pinnacle/input/libinput/{VERSION}/libinput.proto"),
        formatcp!("api/protocol/pinnacle/output/{VERSION}/output.proto"),
        formatcp!("api/protocol/pinnacle/process/{VERSION}/process.proto"),
        formatcp!("api/protocol/pinnacle/tag/{VERSION}/tag.proto"),
        formatcp!("api/protocol/pinnacle/window/{VERSION}/window.proto"),
        formatcp!("api/protocol/pinnacle/window/rules/{VERSION}/rules.proto"),
    ];

    tonic_build::configure()
        .compile(PROTOS, &["api/protocol"])
        .unwrap();
}
