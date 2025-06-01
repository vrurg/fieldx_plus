// Here we test bypassing of some sub-arguments to the upstream `fxstruct` attribute.

use fieldx_plus::fx_plus;

#[fx_plus(parent, builder(post_build))]
struct Foo {
    #[fieldx(inner_mut, get(copy), get_mut)]
    foo: i32,
}

impl Foo {
    fn post_build(self) -> Self {
        *self.foo_mut() += 1;
        self
    }
}

#[test]
fn test_post_build() {
    let foo = Foo::builder().foo(42).build().unwrap();
    assert_eq!(foo.foo(), 43);
}
