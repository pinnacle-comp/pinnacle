use std::path::PathBuf;

fn main() {
    let protobuf_defs_path =
        std::env::var("PINNACLE_PROTOBUF_SNOWCAP_API_DEFS").unwrap_or("../api/protobuf".to_owned());
    println!("cargo:rerun-if-changed={protobuf_defs_path}");

    let mut proto_paths = Vec::new();

    for entry in walkdir::WalkDir::new(&protobuf_defs_path) {
        let entry = entry.unwrap();

        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "proto")
        {
            proto_paths.push(entry.into_path());
        }
    }

    let descriptor_path = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("snowcap.bin");

    tonic_build::configure()
        .file_descriptor_set_path(descriptor_path)
        .boxed(".snowcap.widget.v1.WidgetDef.widget.text_input")
        .compile_protos(&proto_paths, &[protobuf_defs_path])
        .unwrap();
}
