use crate::traits::ProducerDescriptor;

use darling::ast::NestedMeta;
use darling::util::Flag;
use darling::FromMeta;
use fieldx::fxstruct;
use fieldx_aux::validate_exclusives;
use fieldx_aux::FXBool;
use fieldx_aux::FXNestingAttr;
use fieldx_aux::FXOrig;
use fieldx_aux::FXProp;
use fieldx_aux::FXSetState;
use fieldx_aux::FXString;
use fieldx_aux::FXSynTuple;
use fieldx_aux::FromNestAttr;
use proc_macro2::Span;
use proc_macro2::TokenTree;
use quote::format_ident;
use std::marker::PhantomData;
use std::ops::Deref;
use syn::ext::IdentExt;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::Token;

// #[derive(Debug, Clone, Default)]
// pub(crate) struct SlurpyArgs {
//     args: Vec<NestedMeta>,
// }

// impl FromMeta for SlurpyArgs {
//     fn from_list(item: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
//         Ok(Self { args: item.to_vec() })
//     }
// }

// impl ToTokens for SlurpyArgs {
//     fn to_tokens(&self, tokens: &mut TokenStream) {
//         for arg in self.args.iter() {
//             // eprintln!("ARG: {}", indent_all_by(4, format!("{:#?}", arg)));
//             tokens.extend(arg.to_token_stream());
//             tokens.extend(quote![,]);
//         }
//     }
// }

#[derive(FromMeta, Clone, Debug, Default)]
#[fxstruct(default(off), get)]
#[darling(and_then = Self::validate)]
pub(crate) struct UnwrapArg {
    off:         Flag,
    #[darling(rename = "expect")]
    expect_arg:  Option<FXString>,
    #[darling(rename = "or")]
    or_arg:      Option<FXSynTuple<(syn::Path, syn::Expr)>>,
    #[darling(rename = "or_else")]
    or_else_arg: Option<FXSynTuple<(syn::Path, syn::Expr)>>,
}

impl UnwrapArg {
    validate_exclusives! {
        "parent/app drop handling":
            expect_arg as "expect"; or_arg as "or"; or_else_arg as "or_else";
    }

    fn validate(self) -> Result<Self, darling::Error> {
        self.validate_exclusives()?;
        Ok(self)
    }
}

impl FromNestAttr for UnwrapArg {
    fn set_literals(self, literals: &[syn::Lit]) -> darling::Result<Self> {
        self.no_literals(literals)?;
        Ok(self)
    }

    fn for_keyword(_path: &syn::Path) -> darling::Result<Self> {
        Ok(Default::default())
    }
}

impl FXSetState for UnwrapArg {
    fn is_set(&self) -> FXProp<bool> {
        if self.off.is_present() {
            FXProp::new(false, Some(self.off.span()))
        }
        else {
            FXProp::new(true, None)
        }
    }
}

#[fxstruct(get, no_new, default(off), builder)]
#[derive(Debug, Clone)]
pub struct ChildArgsInner<D> {
    parent_type:       syn::Type,
    #[fieldx(optional, get(as_ref))]
    parent_base_ident: syn::Ident,
    #[fieldx(optional, get(as_ref))]
    rc_strong:         FXBool,
    #[fieldx(optional, get(as_ref))]
    unwrap_parent:     FXNestingAttr<UnwrapArg>,
    #[fieldx(builder(off))]
    _d:                PhantomData<D>,
}

#[derive(FromMeta, Debug)]
#[darling(and_then = Self::validate)]
struct _ChldArgs {
    rc_strong:     Option<FXBool>,
    #[darling(rename = "unwrap")]
    unwrap_parent: Option<FXNestingAttr<UnwrapArg>>,
}

impl _ChldArgs {
    validate_exclusives! {"strong/weak parent": rc_strong; unwrap_parent;}

    fn validate(self) -> darling::Result<Self> {
        self.validate_exclusives()?;
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct ChildArgs<D: ProducerDescriptor> {
    inner: ChildArgsInner<D>,
    span:  Span,
}

impl<D: ProducerDescriptor> ChildArgs<D> {
    pub fn span(&self) -> Span {
        self.span
    }

    pub fn parent_base_ident(&self) -> syn::Ident {
        self.inner
            .parent_base_ident()
            .cloned()
            .unwrap_or_else(|| format_ident!("{}", D::base_name(), span = self.span))
    }
}

impl<D: ProducerDescriptor + std::fmt::Debug> Parse for ChildArgs<D> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let Ok(parent_type) = syn::Type::parse(input)
        else {
            return Err(darling::Error::custom(format!("Expected a {} type here", D::kind()))
                .with_span(&input.span())
                .into());
        };

        let mut parent_base_ident = None;

        if input.peek(syn::Token![as]) {
            let _ = input.parse::<Token![as]>()?;
            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Ident::peek_any) {
                let ident = syn::Ident::parse_any(input)?;
                parent_base_ident = Some(ident);
            }
            else {
                return Err(lookahead.error());
            }
        }

        let all_span = input.fork().cursor().token_stream().span();

        input.step(|cursor| {
            let rest = *cursor;
            let Some((tt, next)) = rest.token_tree()
            else {
                return Ok(((), rest));
            };
            if let TokenTree::Punct(ref punct) = tt {
                if punct.as_char() == ',' {
                    return Ok(((), next));
                }
            }
            Err(darling::Error::custom("expected `,`").with_span(&input.span()).into())
        })?;

        let ml = input
            .parse_terminated(NestedMeta::parse, Token![,])?
            .into_iter()
            .collect::<Vec<NestedMeta>>();

        let ca = _ChldArgs::from_list(&ml)?;

        let mut inner_builder = ChildArgsInner::builder().parent_type(parent_type.clone());

        if let Some(base_ident) = parent_base_ident {
            inner_builder = inner_builder.parent_base_ident(base_ident);
        }
        if let Some(rc_strong) = ca.rc_strong {
            inner_builder = inner_builder.rc_strong(rc_strong);
        }
        if let Some(unwrap_parent) = ca.unwrap_parent {
            inner_builder = inner_builder.unwrap_parent(unwrap_parent);
        }

        let inner = inner_builder
            .build()
            .map_err(|err| syn::Error::new(all_span, format!("Error while building arguments struct: {err}")))?;

        Ok(Self { inner, span: all_span })
    }
}

impl<D: ProducerDescriptor> Deref for ChildArgs<D> {
    type Target = ChildArgsInner<D>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use fieldx_aux::FXSetState;
    use quote::quote;
    use quote::ToTokens;

    use crate::codegen::AppDescriptor;

    use super::ChildArgs;

    #[test]
    fn test_child_args() {
        let input = quote! {Node as up_node, unwrap};

        let cargs: ChildArgs<AppDescriptor> = syn::parse2(input).unwrap();

        assert_eq!(cargs.parent_type().to_token_stream().to_string(), "Node");
        assert_eq!(cargs.parent_base_ident().to_string(), "up_node");
        assert!(*cargs.unwrap_parent().is_set());
    }
}
