use darling::{ast::NestedMeta, util::Flag, FromMeta};
use fieldx::fxstruct;
use fieldx_aux::{validate_exclusives, FXNestingAttr, FXStringArg, FXTriggerHelper, FromNestAttr};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Meta;

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
#[fxstruct(default(off), get(clone))]
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

impl FXTriggerHelper for ErrorArg {
    fn is_true(&self) -> bool {
        true
    }
}

#[derive(FromMeta, Clone, Debug, Default)]
#[fxstruct(default(off), get)]
#[darling(and_then = Self::validate)]
pub(crate) struct UnwrapArg {
    off:    Flag,
    expect: Option<FXStringArg>,
    error:  Option<FXNestingAttr<ErrorArg>>,
    map:    Option<FXNestingAttr<ErrorArg>>,
}

impl UnwrapArg {
    validate_exclusives! {
        "drop handling": expect; error; map;
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

impl FXTriggerHelper for UnwrapArg {
    fn is_true(&self) -> bool {
        !self.off().is_present()
    }
}
