use crate::model::{Message, Model, State};

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode};
use std::time::Duration;

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('j') => Some(Message::Increment),
        KeyCode::Char('k') => Some(Message::Decrement),
        KeyCode::Char('q') => Some(Message::Quit),
        _ => None,
    }
}

pub fn handle_event(_: &Model) -> Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

pub fn handle_msg(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Increment => {
            model.counter += 1;
            if model.counter > 50 {
                return Some(Message::Reset);
            }
        }
        Message::Decrement => {
            model.counter -= 1;
            if model.counter < -50 {
                return Some(Message::Reset);
            }
        }
        Message::Reset => model.counter = 0,
        Message::Quit => {
            model.state = State::Quit;
        }
    };
    None
}

pub fn update(model: &mut Model) -> Result<()> {
    let mut current_msg = handle_event(model)?;
    while let Some(msg) = current_msg {
        current_msg = handle_msg(model, msg);
    }
    Ok(())
}
