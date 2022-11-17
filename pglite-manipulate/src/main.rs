use structopt::StructOpt;

mod show_globals;

#[derive(StructOpt)]
enum Cmd {
    ShowGlobals(show_globals::Opt),
}

fn main() -> anyhow::Result<()> {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    );

    match Cmd::from_args() {
        Cmd::ShowGlobals(opt) => show_globals::main(opt),
    }
}
