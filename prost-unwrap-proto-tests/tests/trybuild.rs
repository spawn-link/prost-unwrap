#[test]
#[cfg(feature = "trybuild-tests")]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
