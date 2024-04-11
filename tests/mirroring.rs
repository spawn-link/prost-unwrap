#[prost_unwrap::required(mirror, [])]
pub mod foo {
    pub enum Foo {
        A(bar_one::BarOne),
        B(bar_two::BarTwo),
    }
    pub mod bar_one {
        pub struct BarOne {
            pub a: i32,
        }
        pub mod baz {
            pub struct Baz {
                pub a: i32,
            }
        }
    }
    pub mod bar_two {
        pub struct BarTwo {
            pub a: i32,
        }
        pub mod baz {
            pub struct Baz {
                pub a: i32,
            }
        }
    }
}

#[test]
#[allow(unused_imports)]
// Test that the generated code mirrors the original code
fn success() {
    use mirror::foo::bar_one::baz::Baz as BazOne;
    use mirror::foo::bar_one::BarOne;
    use mirror::foo::bar_two::baz::Baz as BazTwo;
    use mirror::foo::bar_two::BarTwo;
    use mirror::foo::Foo;

    let a = 1;

    let _ = BarOne { a };
    let _ = BarTwo { a };
    let _ = BazOne { a };
    let _ = BazTwo { a };
    let _ = Foo::A(BarOne { a });
    let _ = Foo::B(BarTwo { a });
}
