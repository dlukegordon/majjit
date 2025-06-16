use crate::model::{Model, State};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

const EVENT_POLL_DURATION: Duration = Duration::from_millis(50);

#[derive(PartialEq)]
enum Message {
    Quit,
    SelectNextCommit,
    SelectPrevCommit,
}

pub fn update(model: &mut Model) -> Result<()> {
    let mut current_msg = handle_event(model)?;
    while let Some(msg) = current_msg {
        current_msg = handle_msg(model, msg);
    }
    Ok(())
}

fn handle_event(_: &Model) -> Result<Option<Message>> {
    if event::poll(EVENT_POLL_DURATION)? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                return Ok(handle_key(key));
            }
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Message::Quit),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::SelectNextCommit),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::SelectPrevCommit),
        _ => None,
    }
}

fn handle_msg(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.state = State::Quit;
        }
        Message::SelectNextCommit => {
            let selected = model.log_list_state.selected()?;
            let next = if selected >= model.log_list.len() - 1 {
                0
            } else {
                selected + 1
            };
            model.log_list_state.select(Some(next));
        }
        Message::SelectPrevCommit => {
            let selected = model.log_list_state.selected()?;
            let prev = if selected == 0 {
                model.log_list.len() - 1
            } else {
                selected - 1
            };
            model.log_list_state.select(Some(prev));
        }
    };
    None
}
