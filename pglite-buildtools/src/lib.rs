mod util;

pub mod rewrite_source_globals;
pub mod show_global_symbols;

pub struct LogGuard {
    _scope: slog_scope::GlobalLoggerGuard,
}

pub fn init_logger() -> (slog::Logger, LogGuard) {
    use sloggers::Build;
    use sloggers::terminal::{TerminalLoggerBuilder, Destination};
    use sloggers::types::Severity;

    let log = TerminalLoggerBuilder::new()
        .level(Severity::Info)
        .destination(Destination::Stderr)
        .build()
        .unwrap();

    let scope = slog_scope::set_global_logger(log.clone());

    slog_stdlog::init_with_level(log::Level::Info).unwrap();

    (log, LogGuard { _scope: scope })
}
