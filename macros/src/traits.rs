pub(crate) trait ProducerDescriptor: Clone {
    fn kind() -> &'static str;
    fn base_name() -> &'static str;
    fn child_trait() -> (
        &'static str,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    );
}
