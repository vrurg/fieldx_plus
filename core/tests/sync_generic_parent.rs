use fieldx_plus::{child_build, fx_plus};
use std::sync::Arc;

#[fx_plus(parent, sync, get)]
struct MyParent<T>
where
    T: Default,
{
    v: T,
}

#[fx_plus(child(MyParent<T>, rc_strong), sync, get)]
struct MyChild<T>
where
    T: Default,
{
    v: String,
}

impl<T> MyChild<T>
where
    T: Default + Copy,
{
    fn foo(&self) -> T {
        let p = self.parent();
        *p.v()
    }
}

#[test]
fn generic_parent() {
    let parent: Arc<MyParent<i32>> = MyParent::new();
    assert_eq!(*parent.v(), 0);

    let child = child_build!(parent, MyChild { v: "42?".into() }).expect("Can't create a child object");

    assert_eq!(child.v(), &"42?".to_string());
    assert_eq!(child.foo(), 0);
}
