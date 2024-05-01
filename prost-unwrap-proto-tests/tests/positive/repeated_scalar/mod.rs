pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/repeated_scalar/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::repeated_scalar::generated)
        .with_this_mod(crate::positive::repeated_scalar::sane)
        .with_struct(MsgB, []));
    }
}

#[test]
fn test_conversion() {
    let orig = generated::test::MsgB {
        f1: vec![0.0, 1.0],
        f2: vec![0.0, 1.0],
        f3: vec![0, 1],
        f4: vec![0, 1],
        f5: vec![0, 1],
        f6: vec![0, 1],
        f7: vec![0, 1],
        f8: vec![0, 1],
        f9: vec![0, 1],
        f10: vec![0, 1],
        f11: vec![0, 1],
        f12: vec![0, 1],
        f13: vec![false, true],
        f14: vec!["foo".to_string(), "bar".to_string()],
        f15: vec![vec![0, 1], vec![0, 1]],
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(2, sane.f1.len());
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}
