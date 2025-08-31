mod cli;
mod command_tree;
mod jj_commands;
mod log_tree;
mod model;
mod terminal;
mod update;
mod view;

use std::io::Stdout;

use crate::model::{Model, State};
use crate::update::update;
use crate::view::view;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use jj_commands::JjCommand;
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

fn main() {
    if let Err(err) = _main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn _main() -> Result<()> {
    let args = Args::parse();
    let repository = JjCommand::ensure_valid_repo(&args.repository)?;
    let model = Model::new(repository, args.revisions)?;

    let terminal = terminal::init_terminal()?;
    let result = main_loop(model, terminal);
    terminal::relinquish_terminal()?;

    result
}

fn main_loop(mut model: Model, mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    while model.state != State::Quit {
        terminal.draw(|f| view(&mut model, f))?;
        update(&mut terminal, &mut model)?;
    }
    Ok(())
}
