pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/repeated_struct/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::repeated_struct::generated)
        .with_this_mod(crate::positive::repeated_struct::sane)
        .with_struct(MsgB, []));
    }
}

#[test]
fn test_conversion() {
    let orig = generated::test::MsgB {
        f1: vec![
            generated::test::MsgA { f1: 0 },
            generated::test::MsgA { f1: 1 },
            generated::test::MsgA { f1: 2 },
        ],
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(3, sane.f1.len());
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}
