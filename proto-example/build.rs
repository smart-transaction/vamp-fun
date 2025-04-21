// pub fn main() {
//   tonic_build::configure()
//       .compile_protos(
//           &[
//               "../proto/vamp_fun.proto",
//           ],
//           &["../proto"],
//       )
//       .expect("Failed to compile the proto");
// }

use std::io::Result;
pub fn main() -> Result<()> {
    prost_build::compile_protos(&[
        "../proto/user_objective.proto",
        "../proto/vamp_fun.proto"
    ], &["../proto"])?;
    Ok(())
}