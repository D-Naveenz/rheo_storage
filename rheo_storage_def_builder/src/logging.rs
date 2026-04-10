use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone)]
pub(crate) struct LoggingOptions {
    pub(crate) silent: bool,
    pub(crate) verbose: u8,
    pub(crate) logs_dir: PathBuf,
}

pub(crate) struct LoggingRuntime {
    pub(crate) log_path: PathBuf,
    _guard: WorkerGuard,
}

pub(crate) fn init_logging(options: LoggingOptions) -> Result<LoggingRuntime, std::io::Error> {
    fs::create_dir_all(&options.logs_dir)?;
    let log_path = options.logs_dir.join("rheo_storage_def_builder.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let (writer, guard) = tracing_appender::non_blocking(file);

    let console_max_level = if options.silent {
        LevelFilter::ERROR
    } else {
        match options.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    let file_max_level = match options.verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    let console_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .without_time()
        .with_span_events(FmtSpan::NONE)
        .compact()
        .with_filter(console_max_level);

    let file_layer = fmt::layer()
        .with_writer(writer)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(false)
        .with_span_events(FmtSpan::NONE)
        .compact()
        .with_filter(file_max_level);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .map_err(io_error_from_set_global_default)?;

    Ok(LoggingRuntime {
        log_path,
        _guard: guard,
    })
}

fn io_error_from_set_global_default(
    error: impl std::error::Error + Send + Sync + 'static,
) -> std::io::Error {
    std::io::Error::other(error)
}

#[allow(dead_code)]
pub(crate) fn log_file_path(logs_dir: &Path) -> PathBuf {
    logs_dir.join("rheo_storage_def_builder.log")
}
