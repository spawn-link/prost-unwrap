#[test]
// These tests are executed separately because they obviously fail on Windows
// (see the stderr files for clues)
#[cfg(feature = "ui-tests")]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
