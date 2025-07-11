use crate::ctx::FXPlusCodegenCtx;
use crate::ctx::FXPlusMacroCtx;
use crate::traits::ProducerDescriptor;
use crate::types::ChildArgs;
use darling::FromMeta;
use fieldx::fxstruct;
use fieldx_aux::FXBool;
use fieldx_aux::FXOrig;
use fieldx_aux::FXPropBool;
use fieldx_aux::FXSetState;
use fieldx_aux::FXSpaned;
use fieldx_aux::FXSynValue;
use fieldx_core::codegen::constructor::FXConstructor;
use fieldx_core::codegen::constructor::FXFieldConstructor;
use fieldx_core::codegen::constructor::FXFnConstructor;
use fieldx_core::codegen::constructor::FXImplConstructor;
use fieldx_core::struct_receiver::args::FXStructArgs;
use fieldx_core::struct_receiver::FXStructReceiver;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use quote::ToTokens;
use std::rc::Rc;
use syn::spanned::Spanned;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslateAs {
    Or,
    OrElse,
}

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
    fn child_trait_name(span: Span) -> syn::Ident {
        format_ident!("Agent", span = span)
    }

    #[inline(always)]
    fn rc_assoc_type(span: Span) -> syn::Ident {
        format_ident!("RcApp", span = span)
    }

    #[inline(always)]
    fn weak_assoc_type(span: Span) -> syn::Ident {
        format_ident!("WeakApp", span = span)
    }

    #[inline(always)]
    fn fxp_assoc_type(span: Span) -> syn::Ident {
        format_ident!("FXPApp", span = span)
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
    fn child_trait_name(span: Span) -> syn::Ident {
        format_ident!("Child", span = span)
    }

    #[inline(always)]
    fn rc_assoc_type(span: Span) -> syn::Ident {
        format_ident!("RcParent", span = span)
    }

    #[inline(always)]
    fn weak_assoc_type(span: Span) -> syn::Ident {
        format_ident!("WeakParent", span = span)
    }

    #[inline(always)]
    fn fxp_assoc_type(span: Span) -> syn::Ident {
        format_ident!("FXPParent", span = span)
    }
}

#[fxstruct(get)]
#[derive(FromMeta, Debug, Clone)]
pub(crate) struct FXPlusArgs {
    #[fieldx(optional, get(as_ref))]
    agent:    FXSynValue<ChildArgs<AppDescriptor>>,
    #[fieldx(optional, get(as_ref))]
    app:      FXBool,
    #[fieldx(optional, get(as_ref))]
    parent:   FXBool,
    #[fieldx(optional, get(as_ref))]
    child:    FXSynValue<ChildArgs<ParentDescriptor>>,
    #[darling(flatten)]
    std_args: FXStructArgs,
}

#[fxstruct(new(off))]
pub(crate) struct FXPlusProducer {
    args: FXPlusArgs,
    ctx:  Rc<FXPlusCodegenCtx>,
}

impl FXPlusProducer {
    pub fn new(args: FXPlusArgs, plus_struct: FXStructReceiver) -> Self {
        let impl_ctx = FXPlusMacroCtx::new();
        let ctx = FXPlusCodegenCtx::new(plus_struct, args.std_args.clone(), impl_ctx);
        Self { args, ctx }
    }

    fn ctx(&self) -> &FXPlusCodegenCtx {
        &self.ctx
    }

    fn translate_or_expr(
        &self,
        expr: &syn::Expr,
        translate_as: TranslateAs,
        span: Span,
    ) -> darling::Result<TokenStream> {
        let is_or_else = translate_as == TranslateAs::OrElse;
        let closure_head = if is_or_else {
            quote_spanned! {span=> || }
        }
        else {
            quote! {}
        };

        let expr = match expr {
            syn::Expr::Path(ref path) => {
                if let Some(ident) = path.path.get_ident() {
                    // A single ident path is considered a method name.
                    quote_spanned! {path.span()=> #closure_head self.#ident()}
                }
                else {
                    // A 2+ elements path is used as-is. Normally, it would represent an error enum variant or a
                    // constant with or(); or an Fn reference with or_else().
                    path.to_token_stream()
                }
            }
            syn::Expr::Closure(ref closure) if is_or_else => closure.to_token_stream(),
            _ => {
                let expr_toks = expr.to_token_stream();
                quote_spanned! {span=> #closure_head #expr_toks}
            }
        };
        Ok(expr)
    }

    fn setup_parentish_field<D: ProducerDescriptor>(&self, child_args: &ChildArgs<D>) -> darling::Result<()> {
        let ctx = self.ctx();
        let parent_type = child_args.parent_type();
        let parent_base_ident = child_args.parent_base_ident();
        let builder_name = parent_base_ident.to_string();
        let field_ident = ctx.impl_ctx().parent_field_ident(child_args);
        let rc_strong = child_args.rc_strong().is_set();
        let rc_strong_span = rc_strong.final_span();
        let rc_type = if *rc_strong {
            ctx.impl_details().ref_count_strong(rc_strong_span)
        }
        else {
            ctx.impl_details().ref_count_weak(rc_strong_span)
        };

        let mut field_constructor = FXFieldConstructor::new(
            field_ident,
            quote_spanned! {rc_strong_span=> #rc_type<#parent_type>},
            child_args.span(),
        );

        let mut serde_off = ctx.impl_ctx().serde_off();
        if !serde_off.is_empty() {
            serde_off = quote_spanned! {child_args.span()=> , #serde_off};
        }
        field_constructor.add_attribute_toks(quote_spanned! {child_args.span()=>
            #[fieldx(
                lazy(off), predicate(off), clearer(off), get(off), set(off),
                builder(#builder_name) #serde_off
            )]
        })?;

        ctx.user_struct_mut().add_field(field_constructor);

        Ok(())
    }

    fn setup_unwrapping<D: ProducerDescriptor>(
        &self,
        trait_constructor: &mut FXImplConstructor,
        child_args: &ChildArgs<D>,
    ) -> darling::Result<TokenStream> {
        let ctx = self.ctx();
        let is_rc_strong = child_args.rc_strong().is_set();
        let rc_type_span = child_args
            .rc_strong()
            .map_or_else(|| child_args.span(), |r| r.fx_span());
        let parent_type = child_args.parent_type();
        let rc_strong = ctx.impl_details().ref_count_strong(rc_type_span);
        let rc_weak = ctx.impl_details().ref_count_weak(rc_type_span);
        let rc_assoc = D::rc_assoc_type(child_args.span());
        let weak_assoc = D::weak_assoc_type(child_args.span());
        let fxp_assoc = D::fxp_assoc_type(child_args.span());
        let mut return_type = quote![#rc_strong<#parent_type>];

        trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
            type #weak_assoc = #rc_weak<#parent_type>;
        });

        let fxp_rc_type = if *is_rc_strong { &rc_strong } else { &rc_weak };

        trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
            type #fxp_assoc = #fxp_rc_type<#parent_type>;
        });

        Ok(if let Some(unwrap_arg) = child_args.unwrap_parent() {
            if let Some(expect) = unwrap_arg.expect_arg() {
                let Some(expect_message) = expect.value()
                else {
                    return Err(darling::Error::custom("Missing message for the 'expect' argument").with_span(&expect));
                };

                trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
                    type #rc_assoc = #return_type;
                });

                quote_spanned![expect.span()=> .expect(#expect_message)]
            }
            else if unwrap_arg.or_arg().is_set_bool() || unwrap_arg.or_else_arg().is_set_bool() {
                let Some(or_arg) = unwrap_arg
                    .or_arg()
                    .as_ref()
                    .or_else(|| unwrap_arg.or_else_arg().as_ref())
                else {
                    return Err(darling::Error::custom("Internal error: either `or(...)` or `or_else(...)` subarguments are reported as set, but none contains a value").with_span(&unwrap_arg.final_span()));
                };
                let error_type = or_arg.0.to_token_stream();
                return_type =
                    quote_spanned![or_arg.0.span()=> ::std::result::Result<#rc_strong<#parent_type>, #error_type>];

                trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
                    type #rc_assoc = #return_type;
                });

                if unwrap_arg.or_arg().is_set_bool() {
                    let expr = self.translate_or_expr(&or_arg.1, TranslateAs::Or, or_arg.final_span())?;
                    quote_spanned![or_arg.final_span()=> .ok_or(#expr)]
                }
                else {
                    let expr = self.translate_or_expr(&or_arg.1, TranslateAs::OrElse, or_arg.final_span())?;
                    quote_spanned![or_arg.final_span()=> .ok_or_else(#expr)]
                }
            }
            else {
                trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
                    type #rc_assoc = #return_type;
                });

                quote_spanned![unwrap_arg.final_span()=> .unwrap()]
            }
        }
        else if *is_rc_strong {
            trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
                type #rc_assoc = #return_type;
            });

            quote![]
        }
        else {
            trait_constructor.add_assoc_type(quote_spanned! {parent_type.span()=>
                type #rc_assoc = ::std::option::Option<#return_type>;
            });
            quote![]
        })
    }

    fn setup_child_methods<D: ProducerDescriptor>(
        &self,
        trait_constructor: &mut FXImplConstructor,
        child_args: &ChildArgs<D>,
    ) -> darling::Result<()> {
        let ctx = self.ctx();
        let child_args_span = child_args.span();
        let parent_base_ident = child_args.parent_base_ident();
        let parent_field_ident = ctx.impl_ctx().parent_field_ident(child_args);
        let rc_assoc = D::rc_assoc_type(child_args.span());
        let weak_assoc = D::weak_assoc_type(child_args.span());
        let fxp_assoc = D::fxp_assoc_type(child_args.span());
        let is_rc_strong = child_args.rc_strong().is_set();
        let rc_weak_type = ctx.impl_details().ref_count_weak(child_args_span);

        let final_unwrap = self.setup_unwrapping(trait_constructor, child_args)?;

        let mut parent_method = FXFnConstructor::new(parent_base_ident.clone());
        parent_method
            .set_self_borrow(true)
            .set_span(child_args_span)
            .set_ret_type(quote_spanned! {child_args_span=> Self::#rc_assoc});

        let mut parent_downgrade_method = FXFnConstructor::new(format_ident!(
            "{}_downgrade",
            parent_base_ident,
            span = parent_base_ident.span()
        ));
        parent_downgrade_method
            .set_self_borrow(true)
            .set_span(child_args_span)
            .set_ret_type(quote_spanned! {child_args_span=> Self::#weak_assoc});

        // This method is for the use of macro_rules since its name is not dependent on the user-specified parent name.
        let mut fxplus_parent_method = FXFnConstructor::new_associated(format_ident!(
            "__fxplus_{}",
            D::base_name(),
            span = parent_base_ident.span()
        ));
        fxplus_parent_method
            .set_span(child_args_span)
            .set_ret_type(quote_spanned! {child_args_span=> Self::#fxp_assoc})
            .add_param(quote_spanned! {child_args_span=> #parent_base_ident: Self::#weak_assoc});

        if *is_rc_strong {
            let rc_strong_type = ctx.impl_details().ref_count_strong(child_args_span);

            parent_method
                .set_ret_stmt(quote_spanned! {child_args_span=> #rc_strong_type::clone(&self.#parent_field_ident) });

            parent_downgrade_method.set_ret_stmt(
                quote_spanned! {child_args_span=> #rc_strong_type::downgrade(&self.#parent_field_ident) },
            );

            // unwrap() is safe here because this code is part of app/parent builder macros. Its use outside the macros
            // is at the user's discretion.
            fxplus_parent_method
                .set_ret_stmt(quote_spanned! {child_args_span=> #rc_weak_type::upgrade(&#parent_base_ident).unwrap() });
        }
        else {
            parent_method.set_ret_stmt(
                quote_spanned! {child_args_span=> #rc_weak_type::upgrade(&self.#parent_field_ident) #final_unwrap },
            );

            parent_downgrade_method
                .set_ret_stmt(quote_spanned! {child_args_span=> #rc_weak_type::clone(&self.#parent_field_ident) });

            fxplus_parent_method.set_ret_stmt(quote! {#parent_base_ident});
        }

        trait_constructor
            .add_method(parent_method)
            .add_method(parent_downgrade_method)
            .add_method(fxplus_parent_method);

        Ok(())
    }

    fn impl_childish_trait<D: ProducerDescriptor>(&self, child_args: &ChildArgs<D>) -> darling::Result<()> {
        let ctx = self.ctx();
        let child_args_span = child_args.span();
        let trait_name = D::child_trait_name(child_args_span);
        let mut trait_constructor = FXImplConstructor::new(trait_name);

        trait_constructor
            .set_span(child_args_span)
            .set_from_generics(Some(ctx.input().generics().clone()))
            .set_for_ident(ctx.input_ident());

        self.setup_child_methods(&mut trait_constructor, child_args)?;
        ctx.impl_ctx_mut().add_trait(trait_constructor);

        Ok(())
    }

    fn impl_parent_trait(&self) -> darling::Result<()> {
        let args = &self.args;
        let ctx = self.ctx();

        let trait_name: syn::Path = syn::parse2(quote! { ::fieldx_plus::Parent })?;
        let weak_type = ctx.impl_details().ref_count_weak(args.parent.span());
        let mut trait_constructor = FXImplConstructor::new(trait_name);

        trait_constructor
            .add_assoc_type(quote_spanned! {args.parent.span()=> type WeakSelf = #weak_type<Self>;})
            .set_from_generics(Some(ctx.input().generics().clone()))
            .set_for_ident(ctx.input_ident());

        // Add fx_plus downgrade method
        let mut downgrade_method =
            FXFnConstructor::new(format_ident!("__fxplus_myself_downgrade", span = args.parent.span()));
        downgrade_method
            .set_span(args.parent.span())
            .add_attribute_toks(quote_spanned! {args.parent.span()=> #[inline(always)]})?
            .set_ret_type(quote_spanned! {args.parent.span()=> #weak_type<Self>})
            .set_ret_stmt(quote_spanned! {args.parent.span()=> self.myself_downgrade()});

        trait_constructor.add_method(downgrade_method);

        ctx.impl_ctx_mut().add_trait(trait_constructor);

        Ok(())
    }

    fn setup_struct_as_parentish(&self) -> darling::Result<()> {
        let ctx = self.ctx();
        let args = &self.args;
        let std_args = &self.args.std_args;
        let arg_props = ctx.arg_props();

        // Mark the parentish struct as reference counted unless it is already set.
        if !*arg_props.rc() {
            let is_set_rc = std_args.rc().is_set();
            if !*is_set_rc {
                let is_parent = args.parent.is_set();

                if std_args.rc().is_some() {
                    return Err(darling::Error::custom(format!(
                        "`rc` cannot be disabled for a {} struct",
                        if *is_parent { "parent" } else { "application" }
                    ))
                    .with_span(&is_set_rc.final_span()));
                }

                let myself_name = ctx.impl_ctx().myself_name()?.to_string();
                let parentish_span = args.app.is_set().or(is_parent).final_span();
                ctx.impl_ctx_mut()
                    .add_fxstruct_arg(quote_spanned! {parentish_span=> rc(#myself_name)});
            }
        }

        Ok(())
    }

    fn setup_struct_as_childish(&self) -> darling::Result<()> {
        let ctx = self.ctx();
        let args = &self.args;
        let std_args = &self.args.std_args;
        let arg_props = ctx.arg_props();
        let is_childish = args.child.is_set().or(args.agent.is_set());

        let childish_span = is_childish.final_span();
        if *arg_props.needs_new() {
            let is_set_new = std_args.new().is_set();
            if *is_set_new {
                return Err(
                    darling::Error::custom("`new` argument is not allowed in child/agent context")
                        .with_span(&is_set_new.span()),
                );
            }
            ctx.impl_ctx_mut()
                .add_fxstruct_arg(quote_spanned! {childish_span=> new(off)});
        }

        if !*arg_props.builder().is_set() {
            ctx.impl_ctx_mut()
                .add_fxstruct_arg(quote_spanned! {childish_span=> builder});
        }

        Ok(())
    }

    pub(crate) fn produce(&self) -> darling::Result<TokenStream> {
        let args = &self.args;
        let std_args = &self.args.std_args;
        let ctx = self.ctx();

        let is_app = args.app.is_set();
        let is_parent = args.parent.is_set();
        let is_parentish = is_app.or(is_parent);
        let is_agent = args.agent.is_set();
        let is_child = args.child.is_set();
        let is_childish = is_agent.or(is_child);

        let childish_span = is_childish.final_span();

        // Prevent conflicting run-time mut/immut borrows.
        let serde_off = ctx.impl_ctx().serde_off();
        ctx.impl_ctx_mut().add_fxstruct_arg(serde_off);

        if *is_parentish {
            self.impl_parent_trait()?;
            self.setup_struct_as_parentish()?;
        }

        if *is_childish {
            self.setup_struct_as_childish()?;
        }

        if *is_agent {
            let child_args = args.agent.as_ref().unwrap();
            self.impl_childish_trait(child_args)?;
            self.setup_parentish_field(child_args)?;
        }

        if *is_child {
            let child_args = args.child.as_ref().unwrap();
            self.impl_childish_trait(child_args)?;
            self.setup_parentish_field(child_args)?;
        }

        let mut fxstruct_args = std_args.to_arg_tokens();
        fxstruct_args.extend(ctx.impl_ctx().fxstruct_args().iter().map(|a| a.to_token_stream()));
        let mut struct_constructor = ctx.user_struct_mut();
        struct_constructor.add_attribute_toks(quote_spanned! {childish_span=>
            #[::fieldx::fxstruct( #( #fxstruct_args ),* )]
        })?;
        for trait_constructor in self.ctx().impl_ctx_mut().traits() {
            struct_constructor.add_trait_impl(trait_constructor);
        }
        struct_constructor.add_fields_from_receiver(ctx.input())?;
        let struct_toks = struct_constructor.to_token_stream();

        Ok(quote! {
            use ::fieldx_plus::traits::*;
            #struct_toks
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fieldx_aux::FXSetState;

    #[test]
    fn test_args() {
        let arg_toks = quote![fx_plus(app, parent, rc, lock)];
        let args_meta: syn::Meta = syn::parse2(arg_toks).unwrap();
        let args = FXPlusArgs::from_meta(&args_meta).unwrap();

        assert!(*args.app.is_set());
        assert!(*args.parent.is_set());
        assert!(*args.std_args().rc().is_set());
        assert!(*args.std_args().lock().is_set());
    }
}
