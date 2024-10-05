use darling::{ast, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn;

pub mod agent_producer;
pub mod app_producer;

trait FieldXStruct {
    fn data(&self) -> &ast::Data<(), syn::Field>;

    fn fields(&self) -> Vec<TokenStream> {
        self.data()
            .as_ref()
            .take_struct()
            .unwrap()
            .fields
            .iter()
            .map(|fld| fld.to_token_stream())
            .collect()
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(agent), supports(struct_named), forward_attrs)]
pub(crate) struct AgentStruct {
    pub(crate) vis:      syn::Visibility,
    pub(crate) ident:    syn::Ident,
    pub(crate) data:     ast::Data<(), syn::Field>,
    pub(crate) attrs:    Vec<syn::Attribute>,
    pub(crate) generics: syn::Generics,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(app), supports(struct_named), forward_attrs)]
pub(crate) struct AppStruct {
    pub(crate) vis:      syn::Visibility,
    pub(crate) ident:    syn::Ident,
    pub(crate) data:     ast::Data<(), syn::Field>,
    pub(crate) attrs:    Vec<syn::Attribute>,
    pub(crate) generics: syn::Generics,
}

impl FieldXStruct for AgentStruct {
    fn data(&self) -> &ast::Data<(), syn::Field> {
        &self.data
    }
}

impl FieldXStruct for AppStruct {
    fn data(&self) -> &ast::Data<(), syn::Field> {
        &self.data
    }
}
