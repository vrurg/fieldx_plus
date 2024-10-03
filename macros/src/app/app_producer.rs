use crate::{
    app::{AppStruct, FieldXStruct},
    traits::SyncMode,
    types::SlurpyArgs,
};
use darling::FromMeta;
use fieldx_aux::{FXBoolArg, FXHelper, FXTriggerHelper};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

#[derive(FromMeta, Clone)]
pub(crate) struct AppArgs {
    #[darling(rename = "default")]
    need_default: Option<FXBoolArg>,
    rc:           Option<FXHelper>,
    sync:         Option<FXBoolArg>,
    #[darling(flatten)]
    extra_args:   SlurpyArgs,
}

impl SyncMode for AppArgs {
    fn rc(&self) -> Option<&FXHelper> {
        self.rc.as_ref()
    }

    fn is_sync(&self) -> bool {
        self.sync.as_ref().map_or(false, |b| b.is_true())
    }
}

pub(crate) struct AppProducer {}

impl AppProducer {
    pub fn new() -> Self {
        Self {}
    }

    pub(crate) fn produce(&self, app_struct: AppStruct, args: AppArgs) -> TokenStream {
        let fields = app_struct.fields();
        let AppStruct {
            vis,
            ident,
            attrs,
            generics,
            ..
        } = app_struct;

        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

        let myself_name = args.myself_name();
        let myself_ident = format_ident!("{}", myself_name);
        let myself_downgrade_ident = format_ident!("{}_downgrade", myself_name);
        let mut fxs_args: Vec<TokenStream> = vec![args.sync_arg(), quote![rc(#myself_name)]];

        #[cfg(feature = "serde")]
        fxs_args.push(quote![serde(off)]);

        if args.need_default.as_ref().map_or(true, |nd| nd.is_true()) {
            fxs_args.push(quote![default])
        }

        let extra_args = args.extra_args.to_token_stream();
        if !extra_args.is_empty() {
            fxs_args.push(extra_args);
        }

        let (rc_type, weak_type) = args.rc_type();

        let tt = quote![
            #[::fieldx::fxstruct( #( #fxs_args ),* )]
            #(#attrs)*
            #vis struct #ident #generics #where_clause {
                #( #fields ),*
            }

            impl #impl_generics #ident #type_generics #where_clause {
                #[inline(always)]
                pub fn app(&self) -> #rc_type<Self> {
                    self.#myself_ident().unwrap()
                }

                #[inline(always)]
                pub fn app_downgrade(&self) -> #weak_type<Self> {
                    self.#myself_downgrade_ident()
                }
            }
        ];
        tt
    }
}
