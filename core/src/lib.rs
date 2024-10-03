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
