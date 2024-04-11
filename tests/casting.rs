#[prost_unwrap::required(mirror, ["foo.bar.Bar.a"])]
pub mod foo {
    pub mod bar {
        #[derive(Debug, PartialEq)]
        pub struct Foo {
            pub a: i32,
        }

        #[derive(Debug, PartialEq)]
        pub struct Bar {
            pub a: Option<Foo>,
            pub b: Option<Foo>,
            pub c: Vec<Foo>,
        }
    }
}

#[test]
#[allow(unused_imports)]
// Test that the original struct can be try-casted to the mirror
fn success() {
    use foo::bar::Bar as OrigBar;
    use foo::bar::Foo as OrigFoo;
    use mirror::foo::bar::Bar;
    use mirror::foo::bar::Foo;

    let a = 1;

    let orig = OrigBar {
        a: Some(OrigFoo { a }),
        b: None,
        c: vec![OrigFoo { a }, OrigFoo { a }],
    };
    let sane = Bar {
        a: Foo { a },
        b: None,
        c: vec![Foo { a }, Foo { a }],
    };

    assert_eq!(sane, orig.try_into().unwrap())
}

#[test]
#[allow(unused_imports)]
// Test that the original struct cannot be try-casted to the mirror if the
// required field is missing
fn error() {
    use foo::bar::Bar as OrigBar;
    use foo::bar::Foo as OrigFoo;
    use mirror::foo::bar::Bar;
    use mirror::foo::bar::Foo;

    let orig = OrigBar {
        a: None,
        b: None,
        c: vec![],
    };

    assert_eq!(
        "foo.bar.Bar.a is required",
        <OrigBar as TryInto<Bar>>::try_into(orig)
            .err()
            .unwrap()
            .to_string()
    )
}
