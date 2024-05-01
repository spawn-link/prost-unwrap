pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_enums/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_enums::generated)
        .with_this_mod(crate::positive::oneof_enums::sane)
        .with_struct(MsgB, []));
    }
}

pub mod sane_unwrapped {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_enums/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_enums::generated)
        .with_this_mod(crate::positive::oneof_enums::sane)
        .with_struct(MsgB, [f0]));
    }
}

#[test]
fn test_conversion_wrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(
            generated::test::EnumA1::NonDefault as i32,
        )),
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

#[test]
fn test_conversion_unwrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(
            generated::test::EnumA1::NonDefault as i32,
        )),
    };
    let sane: sane_unwrapped::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

#[test]
#[should_panic]
fn test_error() {
    let orig = generated::test::MsgB { f0: None };
    let _sane: sane_unwrapped::test::MsgB = orig.clone().try_into().unwrap();
}
