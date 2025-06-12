use tonic_build;
use pbjson_build::Builder;
use std::{error::Error, fs};

fn main() -> Result<(), Box<dyn Error>> {
    let proto_files = &["../proto/user_objective.proto", "../proto/vamp_fun.proto"];
    let proto_includes = &["../proto"];

    fs::create_dir_all("src/generated")?; // Be sure that the output directory exists

    let descriptor_path = "src/generated/user_descriptor.pb"; // Output where everything else is

    tonic_build::configure()
        .build_server(true)
        .out_dir("src/generated")
        .file_descriptor_set_path(descriptor_path)
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(proto_files, proto_includes)?;

    let descriptor_bytes = fs::read(descriptor_path)?;
    Builder::new()
        .register_descriptors(&descriptor_bytes)?
        .out_dir("src/generated")
        .extern_path(".vamp.fun", "crate::proto::vamp_fun")
        .build(&[
            ".vamp.fun",
        ])?;

    Ok(())
}
