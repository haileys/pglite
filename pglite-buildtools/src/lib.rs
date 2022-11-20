mod util;

pub mod rewrite_globals;
pub mod show_global_symbols;

use structopt::StructOpt;

#[derive(StructOpt)]
enum Cmd {
    ShowGlobalSymbols(show_global_symbols::Opt),
    RewriteGlobals(rewrite_globals::MainOpt),
    RewriteGlobalsWorker(rewrite_globals::WorkerOpt),
}

pub fn main() -> anyhow::Result<()> {
    let (log, _guard) = init_logger();

    match Cmd::from_args() {
        Cmd::ShowGlobalSymbols(opt) => show_global_symbols::main(log, opt),
        Cmd::RewriteGlobals(opt) => rewrite_globals::main(log, opt),
        Cmd::RewriteGlobalsWorker(opt) => rewrite_globals::worker(log, opt),
    }
}

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
