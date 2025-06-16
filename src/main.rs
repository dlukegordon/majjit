mod cli;
mod jj;
// mod model;
// mod tui;
// mod update;
// mod view;

use jj::Jj;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = cli::Args::parse();
    let jj = Jj::init(args.repository, args.revisions)?;
    Ok(())
}
