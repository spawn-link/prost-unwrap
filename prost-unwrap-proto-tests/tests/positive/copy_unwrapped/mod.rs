pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/copy_unwrapped/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::copy_unwrapped::generated)
        .with_this_mod(crate::positive::copy_unwrapped::sane)
        .with_struct(MsgB, [f1]));
    }
}

#[test]
fn test() {
    let orig = generated::test::MsgB {
        f1: Some(generated::test::MsgA { f1: 0 }),
    };
    let sane: sane::test::MsgB = orig.try_into().unwrap();
    assert_eq!(sane.f1.f1, 0);
    let _orig: generated::test::MsgB = sane.into();
}
