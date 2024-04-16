mod test {
    // NB: trybuild alters the cwd, so the adjustment is needed; actual cwd
    // within trybuild test is
    // $WORKSPACE/target/tests/trybuild/prost-unwrap-proto-tests
    prost_unwrap::include!(from_source(root, InvalidSourceFileArgType));
}

fn main() {}
