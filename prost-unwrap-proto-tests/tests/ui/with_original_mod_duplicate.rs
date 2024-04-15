mod original {}
mod test {
    // NB: trybuild alters the cwd, so the adjustment is needed; actual cwd
    // within trybuild test is
    // $WORKSPACE/target/tests/trybuild/prost-unwrap-proto-tests
    prost_unwrap::include!(from_source(
        "../../../../prost-unwrap-proto-tests/.proto_out/root.inner.rs"
    )
    .with_original_mod(crate::original)
    .with_original_mod(crate::original)
    .with_this_mod(crate::test)
    .with_struct(A, []));
}

fn main() {}
