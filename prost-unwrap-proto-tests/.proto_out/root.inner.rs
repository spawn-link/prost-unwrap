// This file is @generated by prost-build.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct A {
    #[prost(int32, tag = "1")]
    pub f1: i32,
    #[prost(message, optional, tag = "2")]
    pub f2: ::core::option::Option<super::A>,
}
