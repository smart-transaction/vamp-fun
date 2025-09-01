fn main() {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&["../proto/stxn.proto"], &["../proto"]) // shared proto at repo level
        .expect("Failed to compile stxn proto");
} 