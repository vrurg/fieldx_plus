#![cfg(feature = "async")]
use fieldx_plus::agent_builder;
use fieldx_plus::fx_plus;
use std::sync::Arc;

#[fx_plus(app, r#async)]
struct MyApp {
    #[fieldx(lazy, get(clone))]
    foo: String,
}

impl MyApp {
    async fn build_foo(&self) -> String {
        "some str".to_string()
    }
}

#[fx_plus(agent(MyApp, rc_strong), r#async)]
struct AChild {
    #[fieldx(get(clone), builder(into))]
    a_foo: String,
}

impl AChild {
    async fn foo(&self) -> String {
        self.app().foo().await
    }
}

#[tokio::test]
async fn new_app() {
    let app: Arc<MyApp> = MyApp::new();
    assert_eq!(
        app.foo().await,
        "some str".to_string(),
        "lazily initialized using app object"
    );

    let ac = agent_builder!(app, AChild { a_foo: "oki!" })
        .build()
        .expect("Can't create a child object");

    assert_eq!(ac.foo().await, "some str".to_string());
    assert_eq!(ac.a_foo(), "oki!".to_string());
}
