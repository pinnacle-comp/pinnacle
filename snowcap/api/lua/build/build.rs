use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../../protobuf");

    let mut proto_paths = Vec::new();

    for entry in walkdir::WalkDir::new("../../protobuf") {
        let entry = entry.unwrap();

        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "proto")
        {
            proto_paths.push(entry.into_path());
        }
    }

    let descriptor_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("lua-build.bin");

    prost_build::Config::new()
        .file_descriptor_set_path(descriptor_path)
        .compile_protos(&proto_paths, &["../../protobuf"])
        .unwrap();
}
