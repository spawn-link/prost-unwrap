pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/nested_enum/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::nested_enum::generated)
        .with_this_mod(crate::positive::nested_enum::sane)
        .with_struct(MsgB, []));
    }
}

#[test]
fn test_conversion() {
    let orig = generated::test::MsgB {
        f1: generated::test::EnumA::NonDefault as i32,
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}
