use fieldx_aux::{FXHelper, FXHelperTrait};
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) trait SyncMode {
    fn is_sync(&self) -> bool;
    fn rc(&self) -> Option<&FXHelper>;

    fn myself_name(&self) -> String {
        self.rc()
            .and_then(|rc| rc.name().map(|n| n.to_string()))
            .unwrap_or("myself".to_string())
    }

    fn sync_arg(&self) -> TokenStream {
        if self.is_sync() {
            quote![sync]
        }
        else {
            quote![sync(off)]
        }
    }

    fn rc_type(&self) -> (TokenStream, TokenStream) {
        if self.is_sync() {
            (quote![::std::sync::Arc], quote![::std::sync::Weak])
        }
        else {
            (quote![::std::rc::Rc], quote![::std::rc::Weak])
        }
    }
}
