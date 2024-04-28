use std::path::PathBuf;

use const_format::formatcp;

fn main() {
    println!("cargo:rerun-if-changed=./protocol");

    const VERSION: &str = "v0alpha1";
    const PROTOS: &[&str] = &[
        formatcp!("./protocol/pinnacle/{VERSION}/pinnacle.proto"),
        formatcp!("./protocol/pinnacle/input/{VERSION}/input.proto"),
        formatcp!("./protocol/pinnacle/output/{VERSION}/output.proto"),
        formatcp!("./protocol/pinnacle/process/{VERSION}/process.proto"),
        formatcp!("./protocol/pinnacle/tag/{VERSION}/tag.proto"),
        formatcp!("./protocol/pinnacle/window/{VERSION}/window.proto"),
        formatcp!("./protocol/pinnacle/signal/{VERSION}/signal.proto"),
        formatcp!("./protocol/pinnacle/layout/{VERSION}/layout.proto"),
        formatcp!("./protocol/pinnacle/render/{VERSION}/render.proto"),
    ];

    let descriptor_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("pinnacle.bin");

    tonic_build::configure()
        .file_descriptor_set_path(descriptor_path)
        .compile(PROTOS, &["./protocol"])
        .unwrap();
}
