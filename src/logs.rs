use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Sets up the logging configuration for the application.
///
/// This function initializes `tracing_subscriber` with two layers:
/// 1. A layer that logs to stdout (console).
/// 2. A layer that logs to a daily rotating file in the `logs/` directory.
///
/// Log levels are controlled by the `RUST_LOG` environment variable.
/// If `RUST_LOG` is not set, it defaults to `info` for all crates,
/// and `debug` for the `parabellum` crate itself.
pub fn setup_logging() {
    // File appender for daily log rotation
    let file_appender = tracing_appender::rolling::daily("logs", "parabellum.log");
    let (non_blocking_file, _guard_file) = tracing_appender::non_blocking(file_appender);

    // Console layer
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true)
        .with_target(true);

    // File layer
    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false) // No ANSI colors in file logs
        .with_thread_ids(true)
        .with_target(true);

    // Default EnvFilter: info for everything, debug for our crate
    let default_filter = "info,parabellum=debug";

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    // We need to keep the guard alive for the file appender to work
    // A simple way is to leak it. For a more robust solution,
    // you might store it in the App struct or another long-lived object.
    std::mem::forget(_guard_file);
}
