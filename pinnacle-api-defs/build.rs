use std::path::PathBuf;

use const_format::formatcp;

fn main() {
    println!("cargo:rerun-if-changed=../api/protocol");

    const VERSION: &str = "v0alpha1";
    const PROTOS: &[&str] = &[
        formatcp!("../api/protocol/pinnacle/{VERSION}/pinnacle.proto"),
        formatcp!("../api/protocol/pinnacle/input/{VERSION}/input.proto"),
        formatcp!("../api/protocol/pinnacle/output/{VERSION}/output.proto"),
        formatcp!("../api/protocol/pinnacle/process/{VERSION}/process.proto"),
        formatcp!("../api/protocol/pinnacle/tag/{VERSION}/tag.proto"),
        formatcp!("../api/protocol/pinnacle/window/{VERSION}/window.proto"),
        formatcp!("../api/protocol/pinnacle/signal/{VERSION}/signal.proto"),
        formatcp!("../api/protocol/pinnacle/layout/{VERSION}/layout.proto"),
        formatcp!("../api/protocol/pinnacle/render/{VERSION}/render.proto"),
    ];

    let descriptor_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("pinnacle.bin");

    tonic_build::configure()
        .file_descriptor_set_path(descriptor_path)
        .compile(PROTOS, &["../api/protocol"])
        .unwrap();
}
