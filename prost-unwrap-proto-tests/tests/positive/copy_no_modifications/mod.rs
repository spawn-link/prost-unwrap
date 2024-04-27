pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/copy_no_modifications/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::copy_no_modifications::generated)
        .with_this_mod(crate::positive::copy_no_modifications::sane)
        .with_struct(A, []));
    }
}

#[test]
fn struct_copied() {
    let orig = generated::test::A { f1: 0 };
    let sane: sane::test::A = orig.try_into().unwrap();
    let _orig: generated::test::A = sane.into();
}
