use crate::model::{Model, State};

use anyhow::{Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::Duration;

const EVENT_POLL_DURATION: Duration = Duration::from_millis(50);

#[derive(PartialEq)]
enum Message {
    Quit,
    SelectNextLogItem,
    SelectPrevLogItem,
    ToggleLogListFold,
}

pub fn update(model: &mut Model) -> Result<()> {
    let mut current_msg = handle_event(model)?;
    while let Some(msg) = current_msg {
        current_msg = handle_msg(model, msg)?;
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
        KeyCode::Down | KeyCode::Char('j') => Some(Message::SelectNextLogItem),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::SelectPrevLogItem),
        KeyCode::Tab => Some(Message::ToggleLogListFold),
        _ => None,
    }
}

fn handle_msg(model: &mut Model, msg: Message) -> Result<Option<Message>> {
    let list_idx = match model.log_list_state.selected() {
        None => bail!("No log list item is selected"),
        Some(list_idx) => list_idx,
    };

    match msg {
        Message::Quit => {
            model.state = State::Quit;
        }
        Message::SelectNextLogItem => {
            let next = if list_idx >= model.log_list.len() - 1 {
                0
            } else {
                list_idx + 1
            };
            model.log_list_state.select(Some(next));
        }
        Message::SelectPrevLogItem => {
            let prev = if list_idx == 0 {
                model.log_list.len() - 1
            } else {
                list_idx - 1
            };
            model.log_list_state.select(Some(prev));
        }
        Message::ToggleLogListFold => {
            model.toggle_fold(list_idx)?;
        }
    };

    Ok(None)
}
