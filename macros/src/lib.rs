#![allow(non_camel_case_types)]
use darling::{ast, FromDeriveInput, FromMeta};
use proc_macro;
use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::app::{
    agent_producer::{AgentArgs, AgentProducer},
    app_producer::{AppArgs, AppProducer},
    AgentStruct, AppStruct,
};

mod app;
mod traits;
mod types;

macro_rules! gen_tokens {
    ($args:ident as $ty_args:ty, $input:ident as $ty_struct_receiver:ty, $processor:ty) => {{
        let macro_args: $ty_args = match into_attr_args($args) {
            Ok(a) => a,
            Err(e) => return darling::Error::from(e).write_errors().into(),
        };
        let struct_recv: $ty_struct_receiver = match into_struct_receiver($input) {
            Ok(sr) => sr,
            Err(e) => return darling::Error::from(e).write_errors().into(),
        };
        <$processor>::new().produce(struct_recv, macro_args)
    }};
}

fn into_attr_args<ARG_TYPE>(args: proc_macro::TokenStream) -> darling::Result<ARG_TYPE>
where
    ARG_TYPE: FromMeta,
{
    let arg_tokens: TokenStream = args.into();
    let attr_args = ast::NestedMeta::parse_meta_list(arg_tokens.clone())?;
    ARG_TYPE::from_list(&attr_args).map_err(|e| e.with_span(&arg_tokens))
}

fn into_struct_receiver<RECV>(input: proc_macro::TokenStream) -> darling::Result<RECV>
where
    RECV: FromDeriveInput,
{
    let di: DeriveInput = syn::parse::<DeriveInput>(input)?;
    RECV::from_derive_input(&di)
}

#[proc_macro_attribute]
pub fn fx_app(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gen_tokens!(args as AppArgs, input as AppStruct, AppProducer).into()
}

#[proc_macro_attribute]
pub fn fx_agent(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    gen_tokens!(args as AgentArgs, input as AgentStruct, AgentProducer).into()
}
