use fieldx_plus::{agent_builder, fx_plus};
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
}

impl AChild {
    fn foo(&self) -> String {
        self.app().foo()
    }
}

#[test]
fn new_app() {
    let app: Arc<MyApp> = MyApp::new();
    assert_eq!(app.foo(), "some str".to_string());

    let ac = agent_builder!(app, AChild { a_foo: "oki!" })
        .build()
        .expect("Can't create a child object");

    assert_eq!(ac.foo(), "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
