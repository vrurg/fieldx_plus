pub(crate) trait ProducerDescriptor: Clone {
    fn kind() -> &'static str;
    fn base_name() -> &'static str;
}
