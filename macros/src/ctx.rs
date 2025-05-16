use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::rc::Weak;

use darling::Result;
use fieldx::fxstruct;
use fieldx_core::codegen::constructor::FXImplConstructor;
use fieldx_core::ctx::codegen::FXImplementationContext;
use fieldx_core::ctx::FXCodeGenCtx;
use once_cell::unsync::OnceCell;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::ToTokens;

use crate::traits::ProducerDescriptor;
use crate::types::ChildArgs;

pub(crate) type FXPlusCodegenCtx = FXCodeGenCtx<FXPlusMacroCtx>;

#[fxstruct(default(off))]
pub(crate) struct FXPlusMacroCtx {
    codegen_ctx:   Weak<FXCodeGenCtx<Self>>,
    traits:        HashMap<syn::Path, FXImplConstructor>,
    #[fieldx(get)]
    fxstruct_args: Vec<TokenStream>,
    myself_name:   OnceCell<syn::Ident>,
}

impl FXImplementationContext for FXPlusMacroCtx {
    fn set_codegen_ctx(&mut self, ctx: std::rc::Weak<fieldx_core::ctx::FXCodeGenCtx<Self>>) {
        self.codegen_ctx = ctx;
    }
}

impl Debug for FXPlusMacroCtx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FXPlusMacroCtx")
    }
}

impl FXPlusMacroCtx {
    pub(crate) fn codegen_ctx(&self) -> Result<Rc<FXPlusCodegenCtx>> {
        self.codegen_ctx
            .upgrade()
            .ok_or_else(|| darling::Error::custom("Codegen context is lost or not set"))
    }

    pub(crate) fn add_fxstruct_arg(&mut self, args: TokenStream) {
        if !args.is_empty() {
            self.fxstruct_args.push(args);
        }
    }

    pub(crate) fn add_trait(&mut self, constructor: FXImplConstructor) {
        self.traits.insert(constructor.ident().clone(), constructor);
    }

    pub(crate) fn myself_name(&self) -> Result<&syn::Ident> {
        self.myself_name.get_or_try_init(|| {
            let arg_props = self.codegen_ctx()?.arg_props();
            Ok(arg_props
                .myself_name()
                .cloned()
                .unwrap_or_else(|| format_ident!("myself")))
        })
    }

    pub(crate) fn parent_field_ident<D: ProducerDescriptor>(&self, child_args: &ChildArgs<D>) -> syn::Ident {
        let parent_base_ident = child_args.parent_base_ident();
        format_ident!("__{}", parent_base_ident, span = parent_base_ident.span())
    }

    pub(crate) fn traits(&mut self) -> impl Iterator<Item = FXImplConstructor> {
        let mut traits = self.traits.drain().collect::<Vec<_>>();
        traits.sort_by(|a, b| {
            a.0.to_token_stream()
                .to_string()
                .cmp(&b.0.to_token_stream().to_string())
        });
        traits.into_iter().map(|t| t.1)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serde_off(&self) -> TokenStream {
        quote! {serde(off)}
    }

    #[cfg(not(feature = "serde"))]
    pub(crate) fn serde_off(&self) -> TokenStream {
        quote! {}
    }
}
