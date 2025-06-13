mod cli;
mod jj;
mod model;
mod tui;
mod update;
mod view;

use crate::cli::Args;
use crate::jj::Jj;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let args = Args::parse();
    let jj = Jj::load(&args)?;
    jj.get_commits(&args.revisions)?;
    tui::run(&args, jj)?;
    Ok(())
}
