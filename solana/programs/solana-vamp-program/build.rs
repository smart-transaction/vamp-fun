use std::io::Result;
pub fn main() -> Result<()> {
    prost_build::compile_protos(&["../../../proto/vamp_fun.proto"], &["../../../proto"])?;
    Ok(())
}
