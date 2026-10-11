struct Processor {
    name: String,
    enabled: bool,
    callback: Box<
        dyn Fn(&Request) -> Result<Response, Error>
            + Send
            + Sync,
    >,
    retries: usize,
}
