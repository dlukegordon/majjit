use crate::jj::Jj;
use crate::model::{Model, State};
use crate::update::update;
use crate::view::view;

use anyhow::Result;
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    crossterm::{
        ExecutableCommand,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};
use std::{io::stdout, panic};

pub fn run(jj: Jj) -> Result<()> {
    install_panic_hook();
    let terminal = init_terminal()?;

    let res = main_loop(terminal, jj);

    restore_terminal()?;
    res
}

fn main_loop(mut terminal: Terminal<impl Backend>, jj: Jj) -> Result<()> {
    let mut model = Model::new(jj)?;
    while model.state != State::Quit {
        terminal.draw(|f| view(&mut model, f))?;
        update(&mut model)?;
    }
    Ok(())
}

fn init_terminal() -> Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

fn restore_terminal() -> Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal().unwrap();
        original_hook(panic_info);
    }));
}
