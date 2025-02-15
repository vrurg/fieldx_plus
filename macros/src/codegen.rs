use crate::{
    traits::ProducerDescriptor,
    types::{ChildArgs, SlurpyArgs},
};
use darling::{ast, FromDeriveInput, FromMeta};
use fieldx::fxstruct;
use fieldx_aux::{FXBool, FXHelper, FXHelperTrait, FXSynValue, FXTriggerHelper};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::HashMap;
use syn::{spanned::Spanned, Meta};

#[derive(Debug, Clone)]
pub(crate) struct AppDescriptor {}

impl ProducerDescriptor for AppDescriptor {
    #[inline(always)]
    fn kind() -> &'static str {
        "application"
    }

    #[inline(always)]
    fn base_name() -> &'static str {
        "app"
    }

    #[inline(always)]
    fn child_trait() -> (&'static str, TokenStream, TokenStream, TokenStream) {
        ("Agent", quote! {RcApp}, quote! {WeakApp}, quote! {FXPApp})
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParentDescriptor {}

impl ProducerDescriptor for ParentDescriptor {
    #[inline(always)]
    fn kind() -> &'static str {
        "parent"
    }

    #[inline(always)]
    fn base_name() -> &'static str {
        "parent"
    }

    #[inline(always)]
    fn child_trait() -> (&'static str, TokenStream, TokenStream, TokenStream) {
        ("Child", quote! {RcParent}, quote! {WeakParent}, quote! {FXPParent})
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

#[fxstruct(get)]
#[derive(FromMeta, Debug, Clone)]
pub(crate) struct FXPlusArgs {
    #[fieldx(optional, get(as_ref))]
    agent:      FXSynValue<ChildArgs<AppDescriptor>>,
    #[fieldx(optional, get(as_ref))]
    app:        FXBool,
    #[fieldx(optional, get(as_ref))]
    parent:     FXBool,
    #[fieldx(optional, get(as_ref))]
    child:      FXSynValue<ChildArgs<ParentDescriptor>>,
    #[fieldx(optional, predicate, get(as_ref))]
    builder:    SlurpyArgs,
    #[fieldx(optional, get(as_ref))]
    rc:         FXHelper,
    #[fieldx(optional, get(as_ref))]
    sync:       FXBool,
    #[darling(flatten)]
    extra_args: SlurpyArgs,
}

#[fxstruct(no_new)]
pub(crate) struct FXPlusProducer {
    args:        FXPlusArgs,
    plus_struct: FXPlusStruct,

    #[fieldx(inner_mut, get_mut)]
    struct_fields: Vec<TokenStream>,

    #[fieldx(inner_mut, get_mut)]
    traits: HashMap<syn::Type, Vec<TokenStream>>,

    #[fieldx(inner_mut, get_mut)]
    trait_spans: HashMap<syn::Type, Span>,
}

impl FXPlusProducer {
    pub fn new(args: FXPlusArgs, plus_struct: FXPlusStruct) -> Self {
        Self {
            args,
            plus_struct,
            struct_fields: Default::default(),
            traits: Default::default(),
            trait_spans: Default::default(),
        }
    }

    fn add_to_trait<T: ToTokens>(&self, trait_name: &T, tt: TokenStream) -> darling::Result<()> {
        let trait_name: syn::Type = syn::parse2(trait_name.to_token_stream())?;
        let mut traits = self.traits_mut();
        let entry = traits.entry(trait_name.clone()).or_default();
        entry.push(tt);
        Ok(())
    }

    fn set_trait_span<T: ToTokens>(&self, trait_name: &T, span: Span) -> darling::Result<()> {
        let trait_name: syn::Type = syn::parse2(trait_name.to_token_stream())?;
        self.trait_spans_mut().insert(trait_name, span);
        Ok(())
    }

    fn myself_name(&self) -> String {
        self.args
            .rc
            .as_ref()
            .and_then(|rc| rc.name().map(|n| n.to_string()))
            .unwrap_or("myself".to_string())
    }

    fn sync_arg(&self) -> TokenStream {
        self.args
            .sync
            .as_ref()
            .map_or(quote![sync(off)], |b| b.to_token_stream())
    }

    fn rc_type(&self) -> (TokenStream, TokenStream) {
        let span = self.args.sync.as_ref().map_or(Span::call_site(), |s| s.span());
        if self.args.sync.as_ref().map_or(false, |b| b.is_true()) {
            (
                quote_spanned![span=> ::std::sync::Arc],
                quote_spanned![span=> ::std::sync::Weak],
            )
        }
        else {
            (
                quote_spanned![span=> ::std::rc::Rc],
                quote_spanned![span=> ::std::rc::Weak],
            )
        }
    }

    fn child_params<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
    ) -> darling::Result<(TokenStream, TokenStream)> {
        let child_args_span = child_args.span();
        let parent_type = child_args.parent_type();
        let (rc_type, weak_type) = self.rc_type();
        let (trait_name, rc_assoc, weak_assoc, fxp_assoc) = D::child_trait();

        let trait_name = format_ident!("{}", trait_name, span = child_args_span);

        self.add_to_trait(
            &trait_name,
            quote_spanned! {parent_type.span()=>
                type #weak_assoc = #weak_type<#parent_type>;
            },
        )?;

        let fxp_rc_type = if child_args.is_rc_strong() {
            &rc_type
        }
        else {
            &weak_type
        };
        self.add_to_trait(
            &trait_name,
            quote_spanned! {parent_type.span()=>
                type #fxp_assoc = #fxp_rc_type<#parent_type>;
            },
        )?;

        Ok(if let Some(ref unwrap_arg) = child_args.unwrap_parent() {
            let mut return_type = quote![#rc_type<#parent_type>];
            let unwrap_or_error = if let Some(expect) = unwrap_arg.expect_arg() {
                let Some(expect_message) = expect.value()
                else {
                    return Err(darling::Error::custom("Missing message for the 'expect' argument").with_span(&expect));
                };

                self.add_to_trait(
                    &trait_name,
                    quote_spanned! {parent_type.span()=>
                        type #rc_assoc = #rc_type<#parent_type>;
                    },
                )?;

                quote_spanned![expect.span()=> .expect(#expect_message)]
            }
            else if let Some(error) = unwrap_arg.error_arg() {
                let error_type = error.error_type().to_token_stream();
                let expr = error.expr();
                return_type = quote_spanned![error.span()=> Result<#rc_type<#parent_type>, #error_type>];

                self.add_to_trait(
                    &trait_name,
                    quote_spanned! {parent_type.span()=>
                        type #rc_assoc = #return_type;
                    },
                )?;

                quote_spanned![error.span()=> .ok_or(#expr)]
            }
            else if let Some(map) = unwrap_arg.map_arg() {
                let error_type = map.error_type().to_token_stream();
                let span = map.span();
                let expr = match map.expr() {
                    Meta::List(ref call) => quote![.#call],
                    Meta::Path(ref method) => quote![.#method ()],
                    _ => panic!("It's an internal problem: name-value must not appear here!"),
                };
                return_type = quote_spanned![span=> Result<#rc_type<#parent_type>, #error_type>];

                self.add_to_trait(
                    &trait_name,
                    quote_spanned! {parent_type.span()=>
                        type #rc_assoc = #return_type;
                    },
                )?;

                quote_spanned![span=> .ok_or_else(|| self #expr)]
            }
            else {
                self.add_to_trait(
                    &trait_name,
                    quote_spanned! {parent_type.span()=>
                        type #rc_assoc = #rc_type<#parent_type>;
                    },
                )?;

                quote_spanned![unwrap_arg.span()=> .unwrap()]
            };
            (unwrap_or_error, return_type)
        }
        else if child_args.is_rc_strong() {
            self.add_to_trait(
                &trait_name,
                quote_spanned! {parent_type.span()=>
                    type #rc_assoc = #rc_type<#parent_type>;
                },
            )?;
            let span = child_args.rc_strong().map_or_else(Span::call_site, |r| r.span());
            (quote![], quote_spanned![span=> #rc_type<#parent_type>])
        }
        else {
            self.add_to_trait(
                &trait_name,
                quote_spanned! {parent_type.span()=>
                    type #rc_assoc = ::std::option::Option< #rc_type<#parent_type> >;
                },
            )?;
            (quote![], quote![::std::option::Option<#rc_type<#parent_type>>])
        })
    }

    fn parent_field_name<D: ProducerDescriptor>(&self, child_args: &ChildArgs<D>) -> syn::Ident {
        format_ident!("__{}", child_args.base_name(), span = child_args.span())
    }

    fn child_method_bodies<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
        unwrapping: TokenStream,
    ) -> (TokenStream, syn::Ident, TokenStream, TokenStream) {
        let (rc_type, weak_type) = self.rc_type();
        let parent_field_name: syn::Ident = self.parent_field_name(child_args);
        let span = child_args.rc_strong().map_or_else(|| child_args.span(), |r| r.span());
        if child_args.is_rc_strong() {
            (
                rc_type.clone(),
                parent_field_name.clone(),
                quote_spanned! {span=> Arc::clone(&self.#parent_field_name) },
                quote_spanned! {span=> #rc_type::downgrade(&self.#parent_field_name) },
            )
        }
        else {
            (
                weak_type.clone(),
                parent_field_name.clone(),
                quote_spanned! {span=> #weak_type ::upgrade( &self.#parent_field_name ) #unwrapping },
                quote_spanned! {span=> #weak_type ::clone(&self.#parent_field_name) },
            )
        }
    }

    fn child_elems<D: ProducerDescriptor>(
        &self,
        child_args: &ChildArgs<D>,
        serde_off: &Vec<TokenStream>,
    ) -> darling::Result<()> {
        let child_args_span = child_args.span();
        let (unwrapping, _parent_return_type) = self.child_params(child_args)?;
        let base_name = D::base_name();
        let parent_ident = format_ident!("{}", base_name, span = child_args_span);
        let parent_downgrade_ident = format_ident!("{}_downgrade", base_name, span = child_args_span);
        let parent_fxp_ident = format_ident!("__fxplus_{}", base_name, span = child_args_span);
        let parent_type = child_args.parent_type();
        let (rc_field_type, parent_field_name, parent_body, parent_downgrade_body) =
            self.child_method_bodies(child_args, unwrapping);
        self.struct_fields_mut().push(quote_spanned! {child_args_span=>
            #[fieldx(lazy(off), predicate(off), clearer(off), get(off), set(off)
                    #(, #serde_off )*, builder(#base_name))]
            #parent_field_name: #rc_field_type <#parent_type>
        });

        let (trait_name, rc_assoc, weak_assoc, fxp_assoc) = D::child_trait();
        let trait_name = format_ident!("{}", trait_name, span = child_args_span);

        self.set_trait_span(&trait_name, child_args_span)?;

        let fxp_body = if child_args.is_rc_strong() {
            let span = child_args.rc_strong().map_or_else(Span::call_site, |r| r.span());
            let (_, weak_type) = self.rc_type();
            // .unwrap() must be safe here because this code must be part of app/parent builder macros in first place.
            // Any use outside of the macros is at user's disposal!
            quote_spanned! {span=> #weak_type::upgrade(&#parent_ident).unwrap() }
        }
        else {
            quote! {#parent_ident}
        };

        self.add_to_trait(
            &trait_name,
            quote_spanned! {child_args_span=>
                fn #parent_ident(&self) -> Self::#rc_assoc {
                    #parent_body
                }

                fn #parent_downgrade_ident(&self) -> Self::#weak_assoc {
                    #parent_downgrade_body
                }

                fn #parent_fxp_ident(#parent_ident: Self::#weak_assoc) -> Self::#fxp_assoc {
                    #fxp_body
                }
            },
        )?;
        Ok(())
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

        let app_parent_span = args
            .app()
            .map(|a| a.span())
            .or_else(|| args.parent().map(|p| p.span()))
            .unwrap_or_else(Span::call_site);

        if let Some(ref rc) = args.rc() {
            fxs_args.push(rc.to_token_stream());
        }
        else if is_app || is_parent {
            fxs_args.push(quote_spanned! {app_parent_span=> rc(#myself_name)});
        }

        if is_child || is_agent {
            let builder_args = args.builder.as_ref().map_or(quote! {}, |bargs| quote! { (#bargs) });
            let span = args
                .child()
                .map(|c| c.span())
                .or_else(|| args.agent().map(|a| a.span()))
                .unwrap_or_else(Span::call_site);
            fxs_args.push(quote_spanned! {span=> builder #builder_args});
            fxs_args.push(quote_spanned! {span=> no_new});
        }
        else if args.has_builder() {
            let builder_span = args.builder().unwrap().span();
            let builder_args = args.builder().unwrap();
            fxs_args.push(quote_spanned! {builder_span=> builder(#builder_args)});
        }

        let extra_args = args.extra_args.to_token_stream();
        if !extra_args.is_empty() {
            fxs_args.push(extra_args);
        }

        if let Some(ref agent_args) = self.args.agent {
            self.child_elems(agent_args, &serde_off)?;
        }

        if let Some(ref child_args) = self.args.child {
            self.child_elems(child_args, &serde_off)?;
        }

        if is_app || is_parent {
            let (_, weak_type) = self.rc_type();
            let myself_downgrade = format_ident!("{}_downgrade", myself_name);
            let trait_name = format_ident!("{}", "Parent", span = app_parent_span);
            self.set_trait_span(&trait_name, app_parent_span)?;
            self.add_to_trait(
                &trait_name,
                quote_spanned! {app_parent_span=> type WeakSelf = #weak_type<Self>;},
            )?;
            self.add_to_trait(
                &trait_name,
                quote_spanned! {app_parent_span=>
                    #[inline(always)]
                    fn __fxplus_myself_downgrade(&self) -> #weak_type<Self> {
                        self.#myself_downgrade()
                    }
                },
            )?;
        }

        let mut fields = self.struct_fields().clone();
        fields.extend(self.plus_struct.fields());

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let mut trait_impls = vec![];

        for (trait_name, trait_body) in self.traits().iter() {
            let trait_span = self.trait_spans().get(trait_name).unwrap().clone();
            trait_impls.push(quote_spanned! {trait_span=>
                impl #impl_generics ::fieldx_plus::#trait_name for #ident #ty_generics #where_clause {
                    #(#trait_body)*
                }
            });
        }

        Ok(quote! {
            use ::fieldx_plus::traits::*;
            #[::fieldx::fxstruct( #( #fxs_args ),* )]
            #(#attrs)*
            #vis struct #ident #generics #where_clause {
                #( #fields ),*
            }
            #( #trait_impls )*
        })
    }
}
