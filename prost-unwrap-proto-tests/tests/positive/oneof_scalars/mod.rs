pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_scalars/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_scalars::generated)
        .with_this_mod(crate::positive::oneof_scalars::sane)
        .with_struct(MsgB, []));
    }
}

pub mod sane_unwrapped {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_scalars/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_scalars::generated)
        .with_this_mod(crate::positive::oneof_scalars::sane)
        .with_struct(MsgB, [f0]));
    }
}

#[test]
fn test_conversion_wrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(Default::default())),
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

#[test]
fn test_conversion_unwrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(Default::default())),
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
