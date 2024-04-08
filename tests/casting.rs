#[prost_unwrap::required(mirror, ["foo.bar.Bar.a"])]
pub mod foo {
    pub enum A {
        Some(bar::Foo),
        Another(bar::Bar),
        None,
    }

    pub mod bar {
        #[derive(Debug, PartialEq)]
        pub struct Foo {
            pub a: i32,
        }

        #[derive(Debug, PartialEq)]
        pub struct Bar {
            pub a: Option<Foo>,
        }
    }
}

#[test]
#[allow(unused_imports)]
fn success_casting() {
    use foo::bar::Bar as OrigBar;
    use foo::bar::Foo as OrigFoo;
    use mirror::foo::bar::Bar;
    use mirror::foo::bar::Foo;

    let a = 1;

    let orig = OrigBar {
        a: Some(OrigFoo { a }),
    };
    let sane = Bar { a: Foo { a } };

    assert_eq!(sane, orig.try_into().unwrap())
}

#[test]
#[allow(unused_imports)]
fn error_casting() {
    use foo::bar::Bar as OrigBar;
    use foo::bar::Foo as OrigFoo;
    use mirror::foo::bar::Bar;
    use mirror::foo::bar::Foo;

    let orig = OrigBar { a: None };

    assert_eq!(
        "foo.bar.Bar.a is required",
        <foo::bar::Bar as TryInto<Bar>>::try_into(orig)
            .err()
            .unwrap()
            .to_string()
    )
}
