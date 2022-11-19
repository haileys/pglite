mod rewrite_source_globals;
mod show_global_symbols;
mod util;

use structopt::StructOpt;

#[derive(StructOpt)]
enum Cmd {
    ShowGlobalSymbols(show_global_symbols::Opt),
    RewriteSourceGlobals(rewrite_source_globals::Opt)
}

fn main() -> anyhow::Result<()> {
    use sloggers::Build;
    use sloggers::terminal::{TerminalLoggerBuilder, Destination};
    use sloggers::types::Severity;

    let log = TerminalLoggerBuilder::new()
        .level(Severity::Info)
        .destination(Destination::Stderr)
        .build()?;

    let _scope = slog_scope::set_global_logger(log.clone());
    slog_stdlog::init_with_level(log::Level::Info).unwrap();

    match Cmd::from_args() {
        Cmd::ShowGlobalSymbols(opt) => show_global_symbols::main(log, opt),
        Cmd::RewriteSourceGlobals(opt) => rewrite_source_globals::main(log, opt),
    }
}
