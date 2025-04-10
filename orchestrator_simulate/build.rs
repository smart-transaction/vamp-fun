pub fn main() {
    tonic_build::configure()
        .compile_protos(
            &[
                "../proto/user_objective.proto",
            ],
            &["../proto"],
        )
        .expect("Failed to compile state snapshot proto");
}
