#![doc(html_root_url = "https://docs.rs/")]
//! This crate is intended for implementing some design patterns, based on [`fieldx`](https://crates.io/crates/fieldx)
//! crate. At the moment it is only Application/Agent pattern where application object holds global state and agents
//! implement particular aspects of application functionality and, perhaps, interact with each other. One can also
//! consider it somewhat of parent/child relations.  But parent/child pattern is yet to be implemented, so that
//! app/agent can be based upon it.
//!
//! Unfortunately, lack of time doesn't allow me to create comprehensive documentation for this crate. Consider the
//! tests as the points of starting with. But here is, briefly, and idea of how it looks:
//!
//! ```
//! use fieldx_plus::{fx_app, App};
//! use fieldx_plus::{fx_agent, Agent, agent_build};
//! use thiserror::Error;
//!
//! #[derive(Error, Debug)]
//! enum AppError {
//!     #[error("{0}")]
//!     AdHoc(String),
//!     #[error("Application object is unexpectedly gone")]
//!     AppIsGone
//! }
//!
//! #[fx_app(sync)]
//! struct Application {
//!     #[fieldx(lazy, get)]
//!     service: NetService,
//! }
//!
//! impl Application {
//!     fn build_service(&self) -> NetService {
//!         agent_build!(
//!             self.app(), NetService =>
//!             port: 4242;
//!             name: "app service";
//!         ).unwrap()
//!     }
//!
//!     pub fn config(&self, key: &str) -> u32 {
//!         42
//!     }
//!
//!     pub fn run() {
//!         let app = Application::new();
//!         app.service().launch().expect("Something went wrong...");
//!     }
//! }
//!
//! #[fx_agent(Application, sync, unwrap(error(AppError, AppError::AppIsGone)))]
//! struct NetService {
//!     port: u16,
//!     #[fieldx(into, get)]
//!     name: String,
//! }
//!
//! impl NetService {
//!     pub fn launch(&self) -> Result<(), AppError> {
//!         let app = self.app()?;
//!         let cfg = app.config("foo");
//!         println!("Launching '{}' service.", self.name());
//!         Ok(())
//!     }
//! }
//!
//! fn main() {
//!     Application::run();
//! }
//! ```
//!
//! Here is a quick breakdown for it:
//!
//! `fx_app` and `fx_agent` attributes are extenders to `fieldx` `fxstruct` attribute. As such, they take all the
//! arguments, `fxstruct` take. But be aware that they override some of them:
//!
//! - `fx_app` enforces `rc`
//! - `fx_agent` sets `no_new` and `builder`; setting additional parameters of the `builder` are allowed though
//!
//! Both are disabling `serde` if `serde` feature is enabled.
//!
//! Since application struct is always `rc` there is a chance that its reference counter could drop to 0 while an
//! agent attempts to access it. `fx_agent` `unwrap` arguments controls how this situation is handled. It can be
//! a keyword, take `expect`, `error`, or `map` arguments.
//!
//! Two helper macros `agent_build` and `agent_builder` are wrappers around builder pattern. I.e.
//! `agent_build!(self.app(), Agent => foo: 42; bar: "baz")` is actually a shortcut for
//! `Agent::builder().app(self.app()).foo(42).bar("baz").build()`. `agent_builder` is the same but without the final
//! `build()` call.

pub mod traits;

pub use crate::traits::{Agent, App};
pub use fieldx_plus_macros::{fx_agent, fx_app};

#[macro_export]
macro_rules! agent_builder {
    ($self:expr, $($ty:ident)::+ $( => $( $field:ident : $initializer:expr ; )* )? ) => {
            $($ty)::+ ::builder()
                .app( $self.app_downgrade() )
                $( $( .$field($initializer) )* )?
    }
}

#[macro_export]
macro_rules! agent_build {
    ($self:expr, $($ty:ident)::+ $( => $( $field:ident : $initializer:expr ; )* )? ) => {
            $($ty)::+ ::builder()
                .app( $self.app_downgrade() )
                $( $( .$field($initializer) )* )?
                .build()
    }
}
