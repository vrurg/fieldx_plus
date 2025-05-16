use proc_macro2::Span;

pub(crate) trait ProducerDescriptor: Clone {
    fn kind() -> &'static str;
    fn base_name() -> &'static str;
    fn child_trait_name(span: Span) -> syn::Ident;
    fn rc_assoc_type(span: Span) -> syn::Ident;
    fn weak_assoc_type(span: Span) -> syn::Ident;
    fn fxp_assoc_type(span: Span) -> syn::Ident;
}
