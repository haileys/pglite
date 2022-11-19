use structopt::StructOpt;
use pglite::Connection;

use std::io::Read;

#[derive(StructOpt)]
struct Opt {
    #[structopt(short, long)]
    database: std::path::PathBuf,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();

    let conn = Connection::open(&opt.database);

    Ok(())
}
