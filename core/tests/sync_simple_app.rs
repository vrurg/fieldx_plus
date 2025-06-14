#![cfg(feature = "sync")]
use fieldx_plus::agent_builder;
use fieldx_plus::fx_plus;
use std::sync::Arc;

#[fx_plus(app, sync)]
struct MyApp {
    #[fieldx(lazy, get(clone))]
    foo: String,
}

impl MyApp {
    fn build_foo(&self) -> String {
        "some str".to_string()
    }
}

#[fx_plus(agent(MyApp, unwrap), sync)]
struct AChild {
    #[fieldx(get(clone), builder(into))]
    a_foo: String,
    #[fieldx(get(copy))]
    b_foo: i32,
}

impl AChild {
    fn foo(&self) -> String {
        self.app().foo()
    }
}

#[test]
fn new_app() {
    let app: Arc<MyApp> = MyApp::new();
    assert_eq!(app.foo(), "some str".to_string(), "lazily initialized using app object");

    let a_foo = "oki!";
    let b_foo = 42;
    let ac = agent_builder!(app, AChild { a_foo, b_foo })
        .build()
        .expect("Can't create a child object");

    assert_eq!(ac.foo(), "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
    assert_eq!(ac.b_foo(), 42);
}
