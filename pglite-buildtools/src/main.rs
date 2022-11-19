use structopt::StructOpt;

use pglite_buildtools::{show_global_symbols, rewrite_source_globals};

#[derive(StructOpt)]
enum Cmd {
    ShowGlobalSymbols(show_global_symbols::Opt),
    RewriteSourceGlobals(rewrite_source_globals::Opt)
}

fn main() -> anyhow::Result<()> {
    let (log, _guard) = pglite_buildtools::init_logger();

    match Cmd::from_args() {
        Cmd::ShowGlobalSymbols(opt) => show_global_symbols::main(log, opt),
        Cmd::RewriteSourceGlobals(opt) => rewrite_source_globals::main(log, opt),
    }
}
