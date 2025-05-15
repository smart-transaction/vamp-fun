pub fn main() {
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(
            &[
                "../proto/user_objective.proto",
                "../proto/vamp_fun.proto",
            ],
            &["../proto"],
        )
        .expect("Failed to compile state snapshot proto");
}
