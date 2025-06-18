mod cli;
mod jj;
mod model;
mod tui;
mod update;
mod view;

use cli::Args;
use jj::Jj;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = Args::parse();
    let jj = Jj::new(args.repository, args.revisions)?;
    tui::run(jj)?;
    Ok(())
}
