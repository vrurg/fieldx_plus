use fieldx_plus::{agent_build, fx_plus};

trait AppCfg {
    fn cfg(&self) -> String;
}

#[fx_plus(app)]
struct MyApp {}

impl AppCfg for MyApp {
    fn cfg(&self) -> String {
        "app cfg".to_string()
    }
}

#[fx_plus(agent(APP, unwrap))]
struct AChild<APP>
where
    APP: AppCfg, {}

impl<APP> AChild<APP>
where
    APP: AppCfg,
{
    pub fn check_cfg(&self) -> String {
        self.app().cfg()
    }
}

#[test]
fn base() {
    let app = MyApp::new();
    let child = agent_build!(app, AChild<MyApp>).unwrap();
    assert_eq!(child.check_cfg(), "app cfg");
}
