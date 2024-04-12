#[prost_unwrap::required(mirror, ["foo.bar.Bar.a"])]
pub mod foo {
    #[derive(Clone)]
    pub struct Foo {
        pub a: i32,
    }
    pub mod bar {
        pub struct Bar {
            pub a: Option<super::Foo>,
            pub b: Option<super::Foo>,
            pub c: Vec<super::Foo>,
        }
    }
}

#[test]
#[allow(unused_imports)]
// Test that the macro works and unwraps the required field from Option<T>,
// leaving other fields as is
fn success() {
    use mirror::foo::bar::Bar;
    use mirror::foo::Foo;

    let a = 1;

    let foo = Foo { a };
    let _ = Bar {
        a: foo.clone(),
        b: None,
        c: vec![foo.clone()],
    };
}