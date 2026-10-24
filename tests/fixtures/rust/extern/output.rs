extern "Rust" {
    type Context<'host>;
    fn source(self: &Context<'_>, module: &str) -> Result<Text>;

    fn child(
        self: &Context<'_>,
        module: &str,
        name: &str,
        destination: &mut String,
    ) -> Result<Status>;

    fn configuration(self: &Context<'_>, module: &str) -> Result<Configuration>;
}
