#![doc(html_root_url = "https://docs.rs/fieldx_plus")]
//! This crate is intended for implementing some design patterns, based on [`fieldx`](https://crates.io/crates/fieldx)
//! crate. At the moment it is only Application/Agent and Parent/Child patterns. Both are basically the same thing
//! essentially where Application/Parent is a reference counted object and agents/children hold references to it. The
//! difference is in children accessor method names which are `app` and `parent`, correspondingly.
//!
//! Unfortunately, lack of time doesn't allow me to create comprehensive documentation for this crate. Consider the
//! tests as the starting points. But here is, briefly, an idea of how it looks:
//!
//! ```
//! use fieldx_plus::{fx_plus, agent_build};
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
//! # #[cfg(feature = "sync")]
//! #[fx_plus(app, sync)]
//! struct Application {
//!     #[fieldx(lazy, get)]
//!     service: NetService,
//! }
//!
//! # #[cfg(feature = "sync")]
//! impl Application {
//!     fn build_service(&self) -> NetService {
//!         agent_build!(
//!             self, NetService {
//!                 port: 4242,
//!                 name: "app service",
//!             }
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
//! # #[cfg(feature = "sync")]
//! #[fx_plus(agent(Application, unwrap(or(AppError, AppError::AppIsGone))), sync)]
//! struct NetService {
//!     port: u16,
//!     #[fieldx(into, get)]
//!     name: String,
//! }
//!
//! # #[cfg(feature = "sync")]
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
//! # #[cfg(feature = "sync")]
//!     Application::run();
//! }
//! ```
//!
//! Here is a quick breakdown for it:
//!
//! `fx_plus` is an extender to `fieldx::fxstruct` attribute. As such, it takes all the arguments, `fxstruct` takes and
//! adds of couple of its own. But be aware that it overrides some of `fxstruct` arguments:
//!
//! - with `app` or `parent` `fxstruct(rc)` is enforced
//! - with `agent` or `child` it sets `fxstruct(no_new, builder)`; setting additional parameters with the `builder` are
//!   allowed, though: `fx_plus(child, builder(into))`
//!
//! With any of the four `serde` gets disabled if `serde` feature is enabled.
//!
//! Since application/parent struct is always `rc` there is a chance that its reference counter can drop to 0 while an
//! agent/child with weak reference attempts to call `.app()` or `.parent()`, which internally attempt to upgrade the
//! reference. `agent(unwrap)` argument controls how this situation is handled:
//! arguments:
//!
//! - `child(unwrap)` results in simple `.unwrap()` applied the upgrade call
//! - `unwrap(expect("Error message")` commands to use `.expect("Error message")`; with this and the above variant we
//!   always get just the app/parent object
//! - `unwrap(or(ErrorType, <expression>))` would produce `app`/`parent` methods that return `ErrorType`; the
//!   particular value returned depends on the `<expression>`. Say, `ErrorType::ParentIsGone` can be used to return a
//!   specific error code
//! - `unwrap(or_else(ErrorType, <expr>))` can be used to invoke a method on `self`. `<expr>` can either be just method name
//!   or something like `map_to_err("argument", 42)` in which case the `map_to_err` method will get the arguments
//!   specified.
//!
//! Helper macros `agent_build`, `agent_builder`, `child_build`, and `child_builder` are wrappers around builder
//! pattern. I.e.  `agent_build!(self.app(), Agent { foo: 42, bar: "baz" })` is actually a shortcut for
//! `Agent::builder().app(self.app()).foo(42).bar("baz").build()`. `agent_builder` is the same but without the final
//! `build()` call.

pub mod traits;

#[doc(inline)]
pub use crate::traits::Agent;
#[doc(inline)]
pub use crate::traits::Application;
#[doc(inline)]
pub use crate::traits::Child;
#[doc(inline)]
pub use crate::traits::Parent;
#[doc(inline)]
pub use fieldx_plus_macros::fx_plus;

#[macro_export]
#[doc(hidden)]
macro_rules! __fxplus_builder {
    ($method:ident, $conv_method:ident:
        $self:expr, $ty:ty $(
            {
                $( $field:ident $( : $initializer:expr )? ),* $(,)*
            }
        )?
    ) => {
        <$ty>::builder()
            .$method( <$ty>::$conv_method( $self.__fxplus_myself_downgrade() ) )
            $( $( .$field( $crate::__fxplus_builder!(@field_or_expr $field $( : $initializer )? ) ) )* )?
    };

    (@field_or_expr $field:ident : $initializer:expr) => {
        $initializer
    };
    (@field_or_expr $field:ident) => {
        $field
    };
}

#[macro_export]
macro_rules! agent_builder {
    ( $( $args:tt )+ ) => {
        ::fieldx_plus::__fxplus_builder!(app, __fxplus_app: $( $args )+ )
    };
}

#[macro_export]
macro_rules! agent_build {
    ( $( $args:tt )+ ) => {
        ::fieldx_plus::__fxplus_builder!(app, __fxplus_app: $( $args )+ ).build()
    };
}

#[macro_export]
macro_rules! child_builder {
    ( $( $args:tt )+ ) => {
        ::fieldx_plus::__fxplus_builder!(parent, __fxplus_parent: $( $args )+ )
    };
}

#[macro_export]
macro_rules! child_build {
    ( $( $args:tt )+ ) => {
        ::fieldx_plus::__fxplus_builder!(parent, __fxplus_parent: $( $args )+ ).build()
    };
}
