use fieldx_plus::{agent_build, child_build, fx_plus};
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("The app object is gone!")]
    AppGone,
}

impl MyError {
    fn adhoc(_msg: &str) -> Self {
        Self::AppGone
    }
}

#[fx_plus(app, sync(off), builder(into))]
struct MyApp<T>
where
    T: Default,
{
    #[fieldx(lazy, get)]
    foo: T,
}

impl<T> MyApp<T>
where
    T: Default,
{
    fn build_foo(&self) -> T {
        T::default()
    }
}

#[fx_plus(agent(MyApp<String>, unwrap(error(MyError, MyError::adhoc("something")))), parent, sync(off))]
struct AnAgent {
    #[fieldx(get(clone), builder(into))]
    a_foo: String,

    #[fieldx(lazy)]
    child: AChild,
}

impl AnAgent {
    fn foo(&self) -> Result<String, MyError> {
        Ok(self.app()?.foo().clone())
    }

    fn build_child(&self) -> AChild {
        child_build!(self, AChild).expect("Can't create a child object")
    }
}

#[fx_plus(child(AnAgent, unwrap(expect("Parent is unexpectedly gone"))), sync(off))]
struct AChild {
    #[fieldx(get(clone), lazy)]
    b_foo: String,
}

impl AChild {
    fn build_b_foo(&self) -> String {
        format!("b:{}", self.parent().a_foo())
    }
}

#[test]
fn new_app() {
    let app: Rc<MyApp<String>> = MyApp::builder().foo("Foo").build().unwrap();
    assert_eq!(app.foo(), &"Foo".to_string());

    let ac = agent_build!(app, AnAgent { a_foo: "oki!" }).expect("Can't create a child object");

    assert_eq!(ac.child().b_foo(), "b:oki!".to_string());

    assert_eq!(ac.foo().unwrap(), "Foo".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
