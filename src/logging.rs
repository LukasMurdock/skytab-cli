pub fn init_tracing(verbose: u8, stderr_only: bool) {
    let filter = if std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::EnvFilter::from_default_env()
    } else {
        let level = match verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        };
        tracing_subscriber::EnvFilter::new(level)
    };

    if stderr_only {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .without_time()
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .without_time()
            .init();
    }
}
