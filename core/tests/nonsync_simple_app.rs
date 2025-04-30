use fieldx_plus::agent_build;
use fieldx_plus::child_build;
use fieldx_plus::fx_plus;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
enum MyError {
    #[error("The app object is gone!")]
    AppGone,
}

#[fx_plus(app, sync(off))]
struct MyApp {
    #[fieldx(lazy, get(clone))]
    foo: String,
}

impl MyApp {
    fn build_foo(&self) -> String {
        "some str".to_string()
    }
}

#[fx_plus(agent(MyApp, unwrap(or(MyError, no_app))), parent, sync(off))]
struct AnAgent {
    #[fieldx(get(clone), builder(into))]
    a_foo: String,

    #[fieldx(lazy)]
    child: AChild,
}

impl AnAgent {
    fn foo(&self) -> Result<String, MyError> {
        Ok(self.app()?.foo())
    }

    fn build_child(&self) -> AChild {
        child_build!(self, AChild).expect("Can't create a child object")
    }

    fn no_app(&self) -> MyError {
        MyError::AppGone
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
    let app: Rc<MyApp> = MyApp::new();
    assert_eq!(app.foo(), "some str".to_string());

    let ac = agent_build!(app, AnAgent { a_foo: "oki!" }).expect("Can't create a child object");

    assert_eq!(ac.child().b_foo(), "b:oki!".to_string());

    assert_eq!(ac.foo().unwrap(), "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
