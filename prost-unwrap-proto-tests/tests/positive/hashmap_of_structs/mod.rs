pub mod generated {
    pub mod test {
        include!(".proto_out/test.rs");
    }
}

pub mod sane {
    pub mod test {
        prost_unwrap::include!(from_source(
            test,
            "prost-unwrap-proto-tests/tests/positive/hashmap_of_structs/.proto_out/test.rs"
        )
        .with_original_mod(crate::positive::hashmap_of_structs::generated)
        .with_this_mod(crate::positive::hashmap_of_structs::sane)
        .with_struct(MsgB, []));
    }
}

#[test]
fn test_conversion() {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "foo".to_string(),
        generated::test::MsgA {
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
        },
    );
    map.insert(
        "bar".to_string(),
        generated::test::MsgA {
            f1: 1.0,
            f2: 1.0,
            f3: 1,
            f4: 1,
            f5: 1,
            f6: 1,
            f7: 1,
            f8: 1,
            f9: 1,
            f10: 1,
            f11: 1,
            f12: 1,
            f13: true,
            f14: "bar".to_string(),
            f15: vec![5, 4, 3, 2, 1, 0],
        },
    );
    let orig = generated::test::MsgB { f1: map };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}
