#![allow(non_camel_case_types)]

mod codegen;
mod traits;
pub(crate) mod types;

use codegen::FXPlusArgs;
use codegen::FXPlusProducer;
use codegen::FXPlusStruct;
use darling::ast;
use darling::FromDeriveInput;
use darling::FromMeta;
use proc_macro2::TokenStream;
use syn::DeriveInput;

fn into_attr_args<ARG_TYPE>(args: proc_macro::TokenStream) -> darling::Result<ARG_TYPE>
where
    ARG_TYPE: FromMeta,
{
    let arg_tokens: TokenStream = args.into();
    let attr_args = ast::NestedMeta::parse_meta_list(arg_tokens.clone())?;
    ARG_TYPE::from_list(&attr_args).map_err(|e| e.with_span(&arg_tokens))
}

fn into_struct_receiver<RECV>(input: &proc_macro::TokenStream) -> darling::Result<RECV>
where
    RECV: FromDeriveInput,
{
    let di: DeriveInput = syn::parse::<DeriveInput>(input.clone())?;
    RECV::from_derive_input(&di)
}

#[proc_macro_attribute]
pub fn fx_plus(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let macro_args: FXPlusArgs = match into_attr_args(args) {
        Ok(a) => a,
        Err(e) => return e.write_errors().into(),
    };
    let struct_recv: FXPlusStruct = match into_struct_receiver(&input) {
        Ok(sr) => sr,
        Err(e) => return e.write_errors().into(),
    };
    let tt = FXPlusProducer::new(macro_args, struct_recv)
        .produce()
        .unwrap_or_else(|err| err.write_errors());

    tt.into()
}
