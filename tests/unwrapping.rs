#[prost_unwrap::required(mirror, ["foo.bar.Bar.a"])]
pub mod foo {
    pub struct Foo {
        pub a: i32,
    }
    pub mod bar {
        pub struct Bar {
            pub a: Option<super::Foo>,
        }
    }
}

#[test]
#[allow(unused_imports)]
fn mirroring() {
    use mirror::foo::bar::Bar;
    use mirror::foo::Foo;

    let a = 1;

    let foo = Foo { a };
    let _ = Bar { a: foo };
}
