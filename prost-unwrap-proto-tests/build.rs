use std::path::Path;
use std::path::PathBuf;

macro_rules! positive_test {
    ( $case_name:expr ) => {
        let src = PathBuf::from("tests/positive").join($case_name);
        let proto = src.join(".proto/test.proto");
        let includes = src.join(".proto");
        let out_dir = src.join(".proto_out");

        prost_build::Config::new()
            .out_dir(out_dir)
            .compile_protos(&[proto], &[includes])?;
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inner_proto = Path::new("tests/.proto/inner/inner.proto");
    let include_dir = Path::new("tests/.proto");

    prost_build::Config::new()
        .out_dir(".proto_out")
        .compile_protos(&[inner_proto], &[include_dir])?;

    positive_test!("copy_no_modifications");
    positive_test!("copy_unwrapped");

    Ok(())
}
