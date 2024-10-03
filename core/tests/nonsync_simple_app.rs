use fieldx_plus::{agent_builder, fx_agent, fx_app, Agent};
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

#[fx_app(sync(off))]
struct MyApp {
    #[fieldx(lazy, get(clone))]
    foo: String,
}

impl MyApp {
    fn build_foo(&self) -> String {
        "some str".to_string()
    }
}

#[fx_agent(MyApp, sync(off), unwrap(error(MyError, MyError::adhoc("something"))))]
#[derive(Default)]
struct AChild {
    #[fieldx(get(clone), builder(into))]
    a_foo: String,
}

impl AChild {
    fn foo(&self) -> Result<String, MyError> {
        Ok(self.app()?.foo())
    }
}

#[test]
fn new_app() {
    let app: Rc<MyApp> = MyApp::new();
    assert_eq!(app.foo(), "some str".to_string());
    let a: Rc<MyApp> = app.app();
    assert_eq!(a.foo(), "some str".to_string());

    let ac = agent_builder!(
        app, AChild =>
        a_foo: "oki!";
    )
    .build()
    .expect("Can't create a child object");

    assert_eq!(ac.foo().unwrap(), "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
