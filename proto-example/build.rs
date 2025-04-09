pub fn main() {
  tonic_build::configure()
      .compile_protos(
          &[
              "../proto/vamp_fun.proto",
          ],
          &["../proto"],
      )
      .expect("Failed to compile the proto");
}
