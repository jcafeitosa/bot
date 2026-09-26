use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::config::LoggingConfig;

pub fn init(config: &LoggingConfig) -> anyhow::Result<WorkerGuard> {
    std::fs::create_dir_all(Path::new(&config.directory))?;
    let appender = tracing_appender::rolling::daily(&config.directory, "bot.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(writer)
        .with_target(true);
    let stderr_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(true)
        .with_ansi(true);
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init()?;
    Ok(guard)
}
