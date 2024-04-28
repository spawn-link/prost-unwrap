pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/no_modifications/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::no_modifications::generated)
        .with_this_mod(crate::positive::no_modifications::sane)
        .with_struct(A, []));
    }
}

#[test]
fn struct_copied() {
    let orig = generated::test::A { f1: 0 };
    let sane: sane::test::A = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::A>::into(sane));
}
