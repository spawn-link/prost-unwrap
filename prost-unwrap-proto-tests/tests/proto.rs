pub mod generated {
    pub mod root {
        include!("../.proto_out/root.rs");
        pub mod inner {
            include!("../.proto_out/root.inner.rs");
        }
    }
}
pub mod sane {
    pub mod root {
        prost_unwrap::include!(
            from_source(root, "prost-unwrap-proto-tests/.proto_out/root.rs")
                .with_this_mod(crate::sane::root)
                .with_original_mod(crate::generated::root)
                .with_struct(MsgD, [f1])
        );
    }
}

#[test]
fn test() {
    let b = generated::root::EnumB::default();
    assert_eq!(0, b as i32);
}
