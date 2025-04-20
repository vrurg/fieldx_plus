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
use fieldx_aux::FromNestAttr;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use quote::ToTokens;
use std::marker::PhantomData;
use std::ops::Deref;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::Meta;
use syn::Token;

#[derive(Debug, Clone, Default)]
pub(crate) struct SlurpyArgs {
    args: Vec<NestedMeta>,
}

impl FromMeta for SlurpyArgs {
    fn from_list(item: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        Ok(Self {
            args: item.iter().map(|nm| nm.clone()).collect(),
        })
    }
}

impl ToTokens for SlurpyArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for arg in self.args.iter() {
            // eprintln!("ARG: {}", indent_all_by(4, format!("{:#?}", arg)));
            tokens.extend(arg.to_token_stream());
            tokens.extend(quote![,]);
        }
    }
}

#[derive(Clone, Debug)]
#[fxstruct(new(off), default(off), get(clone))]
pub(crate) struct ErrorArg {
    error_type: syn::Path,
    expr:       Meta,
}

impl FromMeta for ErrorArg {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        if items.len() < 2 {
            return Err(darling::Error::custom("Two arguments are expected"));
        }
        if items.len() > 2 {
            return Err(darling::Error::custom("Too many arguments, only two are expected"));
        }
        let NestedMeta::Meta(Meta::Path(error_type)) = items[0].clone()
        else {
            return Err(darling::Error::custom("Expected a error type here").with_span(&items[0]));
        };
        let expr = match items[1] {
            NestedMeta::Meta(ref meta) => {
                if let Meta::NameValue(_) = meta {
                    return Err(darling::Error::custom("Name-value pairs are not supported here").with_span(&items[1]));
                }
                meta.clone()
            }
            _ => {
                return Err(darling::Error::custom("Unsupported expression kind").with_span(&items[1]));
            }
        };
        Ok(Self { error_type, expr })
    }
}

impl FromNestAttr for ErrorArg {
    fn set_literals(self, literals: &Vec<syn::Lit>) -> darling::Result<Self> {
        self.no_literals(literals)?;
        Ok(self)
    }

    fn for_keyword(path: &syn::Path) -> darling::Result<Self> {
        Err(darling::Error::custom("Expected error class as the argument").with_span(&path))
    }
}

impl FXSetState for ErrorArg {
    fn is_set(&self) -> FXProp<bool> {
        FXProp::new(true, None)
    }
}

#[derive(FromMeta, Clone, Debug, Default)]
#[fxstruct(default(off), get)]
#[darling(and_then = Self::validate)]
pub(crate) struct UnwrapArg {
    off:        Flag,
    #[darling(rename = "expect")]
    expect_arg: Option<FXString>,
    #[darling(rename = "error")]
    error_arg:  Option<FXNestingAttr<ErrorArg>>,
    #[darling(rename = "map")]
    map_arg:    Option<FXNestingAttr<ErrorArg>>,
}

impl UnwrapArg {
    validate_exclusives! {
        "drop handling": expect_arg as "expect"; error_arg as "error"; map_arg as "map";
    }

    fn validate(self) -> Result<Self, darling::Error> {
        self.validate_exclusives()?;
        Ok(self)
    }
}

impl FromNestAttr for UnwrapArg {
    fn set_literals(self, literals: &Vec<syn::Lit>) -> darling::Result<Self> {
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

#[fxstruct(get, no_new, default(off))]
#[derive(Debug, Clone)]
pub struct ChildArgsInner<D> {
    parent_type:   syn::Type,
    #[fieldx(optional, get(as_ref))]
    rc_strong:     FXBool,
    #[fieldx(optional, get(as_ref))]
    unwrap_parent: FXNestingAttr<UnwrapArg>,
    _d:            PhantomData<D>,
}

impl<D> ChildArgsInner<D> {
    fn new(parent_type: syn::Type, from_args: _ChldArgs) -> Self {
        Self {
            parent_type,
            rc_strong: from_args.rc_strong,
            unwrap_parent: from_args.unwrap_parent,
            _d: PhantomData::<D>,
        }
    }

    pub fn is_rc_strong(&self) -> bool {
        self.rc_strong().map_or_else(|| false, |rc_strong| *rc_strong.is_set())
    }
}

#[derive(FromMeta, Debug)]
#[darling(and_then = Self::validate)]
struct _ChldArgs {
    rc_strong:     Option<FXBool>,
    #[darling(rename = "unwrap")]
    unwrap_parent: Option<FXNestingAttr<UnwrapArg>>,
}

impl<D: ProducerDescriptor> ChildArgsInner<D> {
    pub(crate) fn base_name(&self) -> &'static str {
        D::base_name()
    }
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
        self.span.clone()
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

        let all = input.fork().cursor().token_stream().span();

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

        Ok(Self {
            inner: ChildArgsInner::new(parent_type, ca),
            span:  all.span(),
        })
    }
}

impl<D: ProducerDescriptor> Deref for ChildArgs<D> {
    type Target = ChildArgsInner<D>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
