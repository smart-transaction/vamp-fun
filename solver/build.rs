pub fn main() {
    tonic_build::configure()
        .compile_protos(&["../proto/state_snapshot.proto"], &["../proto"])
        .expect("Failed to compile state snapshot proto");
}
