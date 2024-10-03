use fieldx_plus::{appobj_builder, fx_app, fx_appobj, AppObj};
use std::sync::Arc;

#[fx_app(sync)]
struct MyApp {
    #[fieldx(lazy, get(clone))]
    foo: String,
}

impl MyApp {
    fn build_foo(&self) -> String {
        "some str".to_string()
    }
}

#[fx_appobj(MyApp, sync, unwrap)]
#[derive(Default)]
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
    let a: Arc<MyApp> = app.app();
    assert_eq!(a.foo(), "some str".to_string());

    let ac = appobj_builder!(
        app, AChild =>
        a_foo: "oki!";
    )
    .build()
    .expect("Can't create a child object");

    assert_eq!(ac.foo(), "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
