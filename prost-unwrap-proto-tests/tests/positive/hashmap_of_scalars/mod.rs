pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/hashmap_of_scalars/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::hashmap_of_scalars::generated)
        .with_this_mod(crate::positive::hashmap_of_scalars::sane)
        .with_struct(MsgB, []));
    }
}

#[test]
fn test_conversion() {
    let mut map = std::collections::HashMap::new();
    map.insert("foo".to_string(), 0);
    map.insert("bar".to_string(), 0);
    let orig = generated::test::MsgB { f1: map };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}
