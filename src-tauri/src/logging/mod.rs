use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Keeps the background log writer alive. Dropping it stops file logging.
pub struct LoggingHandle {
    _guard: WorkerGuard,
}

/// Installs a console + rolling-file subscriber.
///
/// Returns `None` when a subscriber is already installed, which happens in tests.
pub fn install(logs_directory: &Path) -> Option<LoggingHandle> {
    let appender = tracing_appender::rolling::daily(logs_directory, "llama-control.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("LLAMA_CONTROL_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info,llama_control_lib=debug"));

    let installed = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(writer),
        )
        .try_init()
        .is_ok();

    installed.then_some(LoggingHandle { _guard: guard })
}
