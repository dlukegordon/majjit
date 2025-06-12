mod jj;
mod model;
mod update;
mod view;

use crate::jj::Jj;
use crate::model::{Model, State};

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

pub fn init_terminal() -> Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

pub fn restore_terminal() -> Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

pub fn install_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal().unwrap();
        original_hook(panic_info);
    }));
}

fn main() -> Result<()> {
    let jj = Jj::load()?;
    let mut model = Model::new(vec!["line1", "line2", "line3"]);

    install_panic_hook();

    let mut terminal = init_terminal()?;
    while model.state != State::Quit {
        terminal.draw(|f| view::view(&mut model, f))?;
        update::update(&mut model)?;
    }

    restore_terminal()?;
    Ok(())
}
