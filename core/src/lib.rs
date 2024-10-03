pub mod traits;

pub use crate::traits::{App, AppObj};
pub use fieldx_plus_macros::{fx_app, fx_appobj};

#[macro_export]
macro_rules! appobj_builder {
    ($self:expr, $($ty:ident)::+ $( => $( $field:ident : $initializer:expr ; )* )? ) => {
            $($ty)::+ ::builder()
                .app( $self.app_downgrade() )
                $( $( .$field($initializer) )* )?
    }
}

#[macro_export]
macro_rules! appobj_build {
    ($self:expr, $($ty:ident)::+ $( => $( $field:ident : $initializer:expr ; )* )? ) => {
            $($ty)::+ ::builder()
                .app( $self.app_downgrade() )
                $( $( .$field($initializer) )* )?
                .build()
                // .inspect(|appobj| appobj.__fx_app_post_build())
    }
}
