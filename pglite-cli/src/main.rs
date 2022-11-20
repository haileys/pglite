use structopt::StructOpt;
use pglite::Connection;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    database: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let (_log, _scope) = init_logger();
    let opt = Opt::from_args();

    let _conn = Connection::open(&opt.database);

    Ok(())
}

fn init_logger() -> (slog::Logger, slog_scope::GlobalLoggerGuard) {
    use sloggers::Build;
    use sloggers::terminal::{TerminalLoggerBuilder, Destination};
    use sloggers::types::Severity;

    let log = TerminalLoggerBuilder::new()
        .level(Severity::Debug)
        .destination(Destination::Stderr)
        .build()
        .unwrap();

    let scope = slog_scope::set_global_logger(log.clone());

    slog_stdlog::init_with_level(log::Level::Debug).unwrap();

    (log, scope)
}
