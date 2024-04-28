pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/nested_struct/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::nested_struct::generated)
        .with_this_mod(crate::positive::nested_struct::sane)
        .with_struct(MsgB, [f1]));
    }
}

#[test]
fn test_conversion() {
    let orig = generated::test::MsgB {
        f1: Some(generated::test::MsgA { f1: 0 }),
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

#[test]
#[should_panic]
fn test_error() {
    let orig = generated::test::MsgB { f1: None };
    let _sane: sane::test::MsgB = orig.try_into().unwrap();
}
