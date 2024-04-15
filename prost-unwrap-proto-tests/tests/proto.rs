mod proto {
    mod test {
        // include!(concat!(env!("OUT_DIR"), "/root.inner.rs"));
        prost_unwrap::include!(
            from_source("prost-unwrap-proto-tests/.proto_out/root.inner.rs")
                .with_this_mod(crate::foo)
                .with_original_mod(crate::foo)
                .with_struct(root::inner::A, [foo])
                .with_enum(root::inner::B)
        );
    }
}
