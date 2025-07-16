mod cli;
mod jj_commands;
mod jj_log;
mod model;
mod terminal;
mod update;
mod view;

use crate::model::{Model, State};
use crate::update::update;
use crate::view::view;

use anyhow::Result;
use clap::Parser;
use cli::Args;
use ratatui::{Terminal, backend::Backend};

fn main() -> Result<()> {
    let args = Args::parse();
    let repository = jj_commands::ensure_valid_repo(&args.repository)?;
    let terminal = terminal::init_terminal()?;

    let model = Model::new(repository, args.revisions)?;
    let res = main_loop(terminal, model);

    terminal::relinquish_terminal()?;
    res
}

fn main_loop(mut terminal: Terminal<impl Backend>, mut model: Model) -> Result<()> {
    while model.state != State::Quit {
        terminal.draw(|f| view(&mut model, f))?;
        update(&mut terminal, &mut model)?;
    }
    Ok(())
}
