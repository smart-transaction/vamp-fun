pub fn main() {
    tonic_build::configure()
        .compile_protos(
            &[
                "../proto/state_snapshot.proto",
                "../proto/user_objective.proto",
            ],
            &["../proto"],
        )
        .expect("Failed to compile state snapshot proto");
}
