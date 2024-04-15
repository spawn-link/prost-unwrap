use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inner_proto = Path::new("tests/proto/inner/inner.proto");
    let include_dir = Path::new("tests/proto");

    prost_build::Config::new()
        .out_dir(".proto_out")
        .compile_protos(&[inner_proto], &[include_dir])?;

    Ok(())
}
