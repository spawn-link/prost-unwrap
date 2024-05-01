pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_structs/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_structs::generated)
        .with_this_mod(crate::positive::oneof_structs::sane)
        .with_struct(MsgB, []));
    }
}

pub mod sane_unwrapped {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/oneof_structs/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::oneof_structs::generated)
        .with_this_mod(crate::positive::oneof_structs::sane)
        .with_struct(MsgB, [f0]));
    }
}

#[test]
fn test_conversion_wrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(generated::test::MsgA1 {
            f1: 0.0,
            f2: 0.0,
            f3: 0,
            f4: 0,
            f5: 0,
            f6: 0,
            f7: 0,
            f8: 0,
            f9: 0,
            f10: 0,
            f11: 0,
            f12: 0,
            f13: false,
            f14: "foo".to_string(),
            f15: vec![0, 1, 2, 3, 4, 5],
        })),
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

#[test]
fn test_conversion_unwrapped() {
    let orig = generated::test::MsgB {
        f0: Some(generated::test::msg_b::F0::F1(generated::test::MsgA1 {
            f1: 0.0,
            f2: 0.0,
            f3: 0,
            f4: 0,
            f5: 0,
            f6: 0,
            f7: 0,
            f8: 0,
            f9: 0,
            f10: 0,
            f11: 0,
            f12: 0,
            f13: false,
            f14: "foo".to_string(),
            f15: vec![0, 1, 2, 3, 4, 5],
        })),
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
