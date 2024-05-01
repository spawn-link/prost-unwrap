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
    let orig = generated::test::MsgB {
        f1: gen_map::<f64>(),
        f2: gen_map::<f32>(),
        f3: gen_map::<i32>(),
        f4: gen_map::<i64>(),
        f5: gen_map::<u32>(),
        f6: gen_map::<u64>(),
        f7: gen_map::<i32>(),
        f8: gen_map::<i64>(),
        f9: gen_map::<u32>(),
        f10: gen_map::<u64>(),
        f11: gen_map::<i32>(),
        f12: gen_map::<i64>(),
        f13: gen_map::<bool>(),
        f14: gen_map::<String>(),
        f15: gen_map::<Vec<u8>>(),
    };
    let sane: sane::test::MsgB = orig.clone().try_into().unwrap();
    assert_eq!(orig, Into::<generated::test::MsgB>::into(sane));
}

fn gen_map<T>() -> std::collections::HashMap<String, T>
where
    T: Default,
{
    let mut map: std::collections::HashMap<String, T> = std::collections::HashMap::new();
    map.insert("foo".to_string(), Default::default());
    map
}
