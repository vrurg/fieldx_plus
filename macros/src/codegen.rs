use crate::{
    traits::ProducerDescriptor,
    types::{ChildArgs, SlurpyArgs},
};
use darling::{ast, FromDeriveInput, FromMeta};
use fieldx_aux::{FXBoolArg, FXHelper, FXHelperTrait, FXTriggerHelper};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::Meta;

#[derive(Debug, Clone)]
struct AppDescriptor {}

impl ProducerDescriptor for AppDescriptor {
    #[inline(always)]
    fn kind() -> &'static str {
        "application"
    }

    #[inline(always)]
    fn base_name() -> &'static str {
        "app"
    }
}

#[derive(Debug, Clone)]
struct ParentDescriptor {}

impl ProducerDescriptor for ParentDescriptor {
    #[inline(always)]
    fn kind() -> &'static str {
        "parent"
    }

    #[inline(always)]
    fn base_name() -> &'static str {
        "parent"
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named), forward_attrs)]
pub(crate) struct FXPlusStruct {
    pub(crate) vis:      syn::Visibility,
    pub(crate) ident:    syn::Ident,
    pub(crate) data:     ast::Data<(), syn::Field>,
    pub(crate) attrs:    Vec<syn::Attribute>,
    pub(crate) generics: syn::Generics,
}

impl FXPlusStruct {
    fn fields(&self) -> Vec<TokenStream> {
        self.data
            .as_ref()
            .take_struct()
            .unwrap()
            .fields
            .iter()
            .map(|fld| fld.to_token_stream())
            .collect()
    }
}

#[derive(FromMeta, Debug, Clone)]
pub(crate) struct FXPlusArgs {
    agent:         Option<ChildArgs<AppDescriptor>>,
    app:           Option<FXBoolArg>,
    parent:        Option<FXBoolArg>,
    child:         Option<ChildArgs<ParentDescriptor>>,
    builder:       Option<SlurpyArgs>,
    #[darling(rename = "default")]
    needs_default: Option<FXBoolArg>,
    rc:            Option<FXHelper>,
    sync:          Option<FXBoolArg>,
    #[darling(flatten)]
    extra_args:    SlurpyArgs,
}

pub(crate) struct FXPlusProducer {
    args:        FXPlusArgs,
    plus_struct: FXPlusStruct,
}

impl FXPlusProducer {
    pub fn new(args: FXPlusArgs, plus_struct: FXPlusStruct) -> Self {
        Self { args, plus_struct }
    }

    fn myself_name(&self) -> String {
        self.args
            .rc
            .as_ref()
            .and_then(|rc| rc.name().map(|n| n.to_string()))
            .unwrap_or("myself".to_string())
    }

    fn sync_arg(&self) -> TokenStream {
        if self.args.sync.as_ref().map_or(false, |b| b.is_true()) {
            quote![sync]
        }
        else {
            quote![sync(off)]
        }
    }

    fn rc_type(&self) -> (TokenStream, TokenStream) {
        if self.args.sync.as_ref().map_or(false, |b| b.is_true()) {
            (quote![::std::sync::Arc], quote![::std::sync::Weak])
        }
        else {
            (quote![::std::rc::Rc], quote![::std::rc::Weak])
        }
    }

    fn child_params<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
    ) -> darling::Result<(TokenStream, TokenStream)> {
        let parent_type = &child_args.parent_type();
        let (rc_type, _weak_type) = self.rc_type();

        Ok(if let Some(ref unwrap_arg) = child_args.unwrap_parent() {
            let mut return_type = quote![#rc_type<#parent_type>];
            let unwrap_or_error = if let Some(expect) = unwrap_arg.expect_arg() {
                let Some(expect_message) = expect.value()
                else {
                    return Err(darling::Error::custom("Missing message for the 'expect' argument").with_span(&expect));
                };
                quote![.expect(#expect_message)]
            }
            else if let Some(error) = unwrap_arg.error_arg() {
                let error_type = error.error_type().to_token_stream();
                let expr = error.expr();
                return_type = quote![Result<#rc_type<#parent_type>, #error_type>];
                quote![.ok_or(#expr)]
            }
            else if let Some(map) = unwrap_arg.map_arg() {
                let error_type = map.error_type().to_token_stream();
                let expr = match map.expr() {
                    Meta::List(ref call) => quote![.#call],
                    Meta::Path(ref method) => quote![.#method()],
                    _ => panic!("It's an internal problem: name-value must not appear here!"),
                };
                return_type = quote![Result<#rc_type<#parent_type>, #error_type>];
                quote![.ok_or_else(|| self #expr)]
            }
            else {
                quote![.unwrap()]
            };
            (unwrap_or_error, return_type)
        }
        else {
            (quote![], quote![::std::option::Option<#rc_type<#parent_type>>])
        })
    }

    fn parent_field_name<D: ProducerDescriptor>(&self, child_args: &ChildArgs<D>) -> syn::Ident {
        format_ident!("__{}", child_args.base_name())
    }

    fn child_method_bodies<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
        unwrapping: TokenStream,
    ) -> (TokenStream, syn::Ident, TokenStream, TokenStream) {
        let (rc_type, weak_type) = self.rc_type();
        let parent_field_name: syn::Ident = self.parent_field_name(child_args);
        if child_args.rc_strong().map_or(false, |rs| rs.is_true()) {
            (
                rc_type.clone(),
                parent_field_name.clone(),
                quote! { self.#parent_field_name },
                quote! { #rc_type::downgrade(&self.#parent_field_name) },
            )
        }
        else {
            (
                weak_type.clone(),
                parent_field_name.clone(),
                quote! { self.#parent_field_name.upgrade()#unwrapping },
                quote! { #weak_type ::clone(&self.#parent_field_name) },
            )
        }
    }

    fn child_elems<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
        serde_off: &Vec<TokenStream>,
    ) -> darling::Result<(TokenStream, TokenStream)> {
        let (unwrapping, parent_return_type) = self.child_params(child_args)?;
        let base_name = D::base_name();
        let parent_ident = format_ident!("{}", base_name);
        let parent_downgrade_ident = format_ident!("{}_downgrade", base_name);
        let parent_type = child_args.parent_type();
        let (rc_field_type, parent_field_name, parent_body, parent_downgrade_body) =
            self.child_method_bodies(child_args, unwrapping);
        let (_, weak_type) = self.rc_type();
        Ok((
            quote! {
                #[fieldx(lazy(off), predicate(off), clearer(off), get(off), set(off)
                        #(, #serde_off )*, builder(#base_name))]
                #parent_field_name: #rc_field_type <#parent_type>
            },
            quote! {
                fn #parent_ident(&self) -> #parent_return_type {
                    #parent_body
                }

                fn #parent_downgrade_ident(&self) -> #weak_type <#parent_type> {
                    #parent_downgrade_body
                }
            },
        ))
    }

    pub(crate) fn produce(&self) -> darling::Result<TokenStream> {
        let FXPlusStruct {
            ident,
            generics,
            vis,
            attrs,
            ..
        } = &self.plus_struct;

        let args = &self.args;

        let is_app = args.app.as_ref().map_or(false, |b| b.is_true());
        let is_parent = args.parent.as_ref().map_or(false, |b| b.is_true());
        let is_agent = args.agent.is_some();
        let is_child = args.child.is_some();

        let myself_name = self.myself_name();

        let mut fxs_args: Vec<TokenStream> = vec![self.sync_arg()];

        #[cfg(feature = "serde")]
        let serde_off = vec![quote![serde(off)]];
        #[cfg(not(feature = "serde"))]
        let serde_off: Vec<TokenStream> = vec![];

        #[cfg(feature = "serde")]
        fxs_args.extend(serde_off.clone());

        if let Some(ref rc) = args.rc {
            fxs_args.push(rc.to_token_stream());
        }
        else if is_app || is_parent {
            fxs_args.push(quote! {rc(#myself_name)});
        }

        if is_child || is_agent {
            let builder_args = args.builder.as_ref().map_or(quote! {}, |bargs| quote! { (#bargs) });
            fxs_args.push(quote! {builder #builder_args});
            fxs_args.push(quote! {no_new});
        }

        if args.needs_default.as_ref().map_or(true, |nd| nd.is_true()) {
            fxs_args.push(quote! {default});
        }

        let extra_args = args.extra_args.to_token_stream();
        if !extra_args.is_empty() {
            fxs_args.push(extra_args);
        }

        let mut methods = vec![];
        let mut fields = vec![];

        if let Some(ref app_args) = self.args.agent {
            let (app_field, agent_methods) = self.child_elems(app_args, &serde_off)?;
            fields.push(app_field);
            methods.push(agent_methods);
        }

        if let Some(ref child_args) = self.args.child {
            let (parent_field, child_methods) = self.child_elems(child_args, &serde_off)?;
            fields.push(parent_field);
            methods.push(child_methods);
        }

        if is_app || is_parent {
            let (_, weak_type) = self.rc_type();
            let myself_downgrade = format_ident!("{}_downgrade", myself_name);
            methods.push(quote! {
                #[inline(always)]
                fn __fxplus_myself_downgrade(&self) -> #weak_type<Self> {
                    self.#myself_downgrade()
                }
            })
        }

        fields.extend(self.plus_struct.fields());

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let struct_impl = if methods.len() > 0 {
            quote! {
                impl #impl_generics #ident #ty_generics #where_clause {
                    #(#methods)*
                }
            }
        }
        else {
            quote! {}
        };

        Ok(quote! {
            #[::fieldx::fxstruct( #( #fxs_args ),* )]
            #(#attrs)*
            #vis struct #ident #generics #where_clause {
                #( #fields ),*
            }
            #struct_impl
        })
    }
}
