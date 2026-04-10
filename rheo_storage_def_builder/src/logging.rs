use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LoggingOptions {
    pub(crate) silent: bool,
    pub(crate) verbose: u8,
}

pub(crate) fn init_logging(options: LoggingOptions) {
    let max_level = if options.silent {
        LevelFilter::ERROR
    } else {
        match options.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    let _ = fmt()
        .with_max_level(max_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_span_events(FmtSpan::NONE)
        .compact()
        .try_init();
}
