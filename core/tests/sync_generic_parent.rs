#![cfg(feature = "sync")]
use fieldx_plus::child_builder;
use fieldx_plus::fx_plus;
use std::sync::Arc;

#[fx_plus(parent, sync, get, builder(into))]
struct MyParent<T, S>
where
    T: Default,
    S: Default,
{
    v:  T,
    v2: S,
}

#[fx_plus(child(MyParent<T, S>, rc_strong), sync, get)]
struct MyChild<T, S>
where
    T: Default,
    S: Default,
{
    v: String,
}

impl<T, S> MyChild<T, S>
where
    T: Default + Copy,
    S: Default + Clone,
{
    fn foo(&self) -> T {
        let p = self.parent();
        *p.v()
    }

    fn bar(&self) -> S {
        self.parent().v2().clone()
    }
}

#[test]
fn generic_parent() {
    let parent: Arc<MyParent<i32, String>> = MyParent::builder().v(42).v2("The Answer").build().unwrap();
    assert_eq!(*parent.v(), 42);

    let child = child_builder!(parent, MyChild::<i32, String> { v: "42?".into() })
        .build()
        .expect("Can't create a child object");

    assert_eq!(child.v(), &"42?".to_string());
    assert_eq!(child.bar(), "The Answer".to_string());
    assert_eq!(child.foo(), 42);
}
