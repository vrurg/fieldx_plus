use crate::{
    app::{AgentStruct, FieldXStruct},
    traits::SyncMode,
    types::{SlurpyArgs, UnwrapArg},
};
use darling::{ast::NestedMeta, FromMeta};
use fieldx_aux::{FXBoolArg, FXHelper, FXNestingAttr, FXOrig, FXStringArg, FXTriggerHelper};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens};
use syn::{spanned::Spanned, Meta};

#[derive(Clone)]
pub(crate) struct AgentArgs {
    app_type:   Meta,
    sync:       Option<FXBoolArg>,
    rc:         Option<FXHelper>,
    unwrap:     Option<FXNestingAttr<UnwrapArg>>,
    builder:    Option<SlurpyArgs>,
    post_build: Option<FXStringArg>,
    extra_args: SlurpyArgs,
}

#[derive(FromMeta, Debug)]
struct _AOA {
    sync:       Option<FXBoolArg>,
    rc:         Option<FXHelper>,
    unwrap:     Option<FXNestingAttr<UnwrapArg>>,
    builder:    Option<SlurpyArgs>,
    post_build: Option<FXStringArg>,
    #[darling(flatten)]
    extra_args: SlurpyArgs,
}

impl FromMeta for AgentArgs {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        if items.len() < 1 {
            return Err(darling::Error::custom(
                "Expected a parent application type as the first argument",
            ));
        }
        let NestedMeta::Meta(app_type) = items[0].clone()
        else {
            return Err(darling::Error::custom("Expected a parent application type here").with_span(&items[0]));
        };
        let rest = &items[1..];

        let aoa = _AOA::from_list(rest)?;

        Ok(Self {
            app_type,
            sync: aoa.sync,
            rc: aoa.rc,
            unwrap: aoa.unwrap,
            builder: aoa.builder,
            extra_args: aoa.extra_args,
            post_build: aoa.post_build,
        })
    }
}

impl SyncMode for AgentArgs {
    fn rc(&self) -> Option<&FXHelper> {
        self.rc.as_ref()
    }

    fn is_sync(&self) -> bool {
        self.sync.as_ref().map_or(false, |b| b.is_true())
    }
}

impl AgentArgs {
    pub(crate) fn needs_unwrap(&self) -> bool {
        self.unwrap.as_ref().map_or(false, |u| u.is_true())
    }
}

pub(crate) struct AgentProducer {}

impl AgentProducer {
    pub fn new() -> Self {
        Self {}
    }

    pub(crate) fn produce(&self, agent_struct: AgentStruct, args: AgentArgs) -> TokenStream {
        let fields = agent_struct.fields();
        let AgentStruct {
            vis,
            ident,
            attrs,
            generics,
            ..
        } = agent_struct;

        let app_field_name = format_ident!("__app");
        let app_type = &args.app_type;
        let builder_args = args.builder.as_ref().map_or(quote![], |b| b.to_token_stream());

        #[cfg(feature = "serde")]
        let serde_off = vec![quote![serde(off)]];
        #[cfg(not(feature = "serde"))]
        let serde_off: Vec<TokenStream> = vec![];

        let mut fxs_args: Vec<proc_macro2::TokenStream> =
            vec![args.sync_arg(), quote![no_new], quote![builder(#builder_args)]];

        #[cfg(feature = "serde")]
        fxs_args.extend(serde_off.clone());

        if let Some(rc) = args.rc() {
            fxs_args.push(rc.orig().map_or(quote![], |meta| meta.to_token_stream()));
        }

        // Must always be the last push because extra_args comes with a trailing comma which causes a compile error.
        let extra_args = (&args.extra_args).to_token_stream();
        if !extra_args.is_empty() {
            fxs_args.push(extra_args);
        }

        let (rc_type, weak_type) = args.rc_type();

        let self_arg = if args.rc.as_ref().map_or(false, |rc| rc.is_true()) {
            quote![self: &#rc_type <Self>]
        }
        else {
            quote![&self]
        };

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let post_build_tt = if let Some(ref post_build) = args.post_build {
            let span = post_build.orig().span();
            let post_build_method = format_ident!("{}", post_build.value().unwrap());
            quote_spanned![span=>
                #[inline(always)]
                pub fn __fx_app_post_build(#self_arg) {
                    self.#post_build_method();
                }
            ]
        }
        else {
            quote![
                #[inline(always)]
                pub fn __fx_app_post_build(#self_arg) {}
            ]
        };

        let (unwrapping, app_return_type) = if args.needs_unwrap() {
            let unwrap_arg = args.unwrap.as_ref().unwrap();
            let mut return_type = quote![#rc_type<#app_type>];
            let unwrap_or_error = if let Some(expect) = unwrap_arg.expect() {
                let expect_message = expect.value().expect("Missing a message for the 'expect' argument");
                quote![.expect(#expect_message)]
            }
            else if let Some(error) = unwrap_arg.error() {
                let error_type = error.error_type().to_token_stream();
                let expr = error.expr();
                return_type = quote![Result<#rc_type<#app_type>, #error_type>];
                quote![.ok_or(#expr)]
            }
            else if let Some(map) = unwrap_arg.map() {
                let error_type = map.error_type().to_token_stream();
                let expr = match map.expr() {
                    Meta::List(ref call) => quote![.#call],
                    Meta::Path(ref method) => quote![.#method()],
                    _ => panic!("It's an internal problem: name-value must not appear here!"),
                };
                return_type = quote![Result<#rc_type<#app_type>, #error_type>];
                quote![.ok_or_else(|| self #expr)]
            }
            else {
                quote![.unwrap()]
            };
            (unwrap_or_error, return_type)
        }
        else {
            (quote![], quote![::std::option::Option<#rc_type<#app_type>>])
        };

        let tt = quote![
            #[::fieldx::fxstruct( #( #fxs_args ),* )]
            #(#attrs)*
            #vis struct #ident #generics #where_clause {
                #[fieldx(lazy(off), predicate(off), clearer(off), get(off), set(off)
                         #(, #serde_off )*, builder("app"))]
                #app_field_name: #weak_type <#app_type>,
                #( #fields ),*
            }

            impl #impl_generics #ident #ty_generics #where_clause {
                #post_build_tt
            }

            impl #impl_generics ::fieldx_plus::Agent<#rc_type <Self>> for #ident #ty_generics #where_clause {
                type AppType = #app_return_type;
                type WeakType = #weak_type <#app_type>;
                fn app(&self) -> Self::AppType {
                    self.#app_field_name.upgrade()#unwrapping
                }

                fn app_downgrade(&self) -> Self::WeakType {
                    #weak_type ::clone(&self.#app_field_name)
                }
            }
        ];
        tt
    }
}
