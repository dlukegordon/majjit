use crate::model::{Model, State};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::{Terminal, backend::Backend};
use std::time::Duration;

const EVENT_POLL_DURATION: Duration = Duration::from_millis(50);

#[derive(Debug, PartialEq)]
pub enum Message {
    Quit,
    SelectNextNode,
    SelectPrevNode,
    SelectParentNode,
    SelectNextSiblingNode,
    SelectPrevSiblingNode,
    ToggleLogListFold,
    ScrollDown,
    ScrollUp,
    ScrollDownPage,
    ScrollUpPage,
    Refresh,
    Describe,
    New,
    Abandon,
    Undo,
    Commit,
    Squash,
    Edit,
    Fetch,
    Push,
}

pub fn update(terminal: &mut Terminal<impl Backend>, model: &mut Model) -> Result<()> {
    let mut current_msg = handle_event(model)?;

    while let Some(msg) = current_msg {
        current_msg = handle_msg(terminal, model, msg)?;
    }
    Ok(())
}

fn handle_event(_: &Model) -> Result<Option<Message>> {
    if event::poll(EVENT_POLL_DURATION)? {
        match event::read()? {
            Event::Key(key) => {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(handle_key(key));
                }
            }
            Event::Mouse(mouse) => {
                return Ok(handle_mouse(mouse));
            }
            _ => {}
        }
    }
    Ok(None)
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Message::Quit),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::SelectNextNode),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::SelectPrevNode),
        KeyCode::PageDown => Some(Message::ScrollDownPage),
        KeyCode::PageUp => Some(Message::ScrollUpPage),
        KeyCode::Left | KeyCode::Char('h') => Some(Message::SelectPrevSiblingNode),
        KeyCode::Right | KeyCode::Char('l') => Some(Message::SelectNextSiblingNode),
        KeyCode::Char('K') => Some(Message::SelectParentNode),
        KeyCode::Tab => Some(Message::ToggleLogListFold),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Refresh)
        }
        KeyCode::Char('d') => Some(Message::Describe),
        KeyCode::Char('n') => Some(Message::New),
        KeyCode::Char('a') => Some(Message::Abandon),
        KeyCode::Char('u') => Some(Message::Undo),
        KeyCode::Char('c') => Some(Message::Commit),
        KeyCode::Char('s') => Some(Message::Squash),
        KeyCode::Char('e') => Some(Message::Edit),
        KeyCode::Char('f') => Some(Message::Fetch),
        KeyCode::Char('p') => Some(Message::Push),
        _ => None,
    }
}

fn handle_mouse(mouse: event::MouseEvent) -> Option<Message> {
    match mouse.kind {
        MouseEventKind::ScrollDown => Some(Message::ScrollDown),
        MouseEventKind::ScrollUp => Some(Message::ScrollUp),
        _ => None,
    }
}

fn handle_msg(
    term: &mut Terminal<impl Backend>,
    model: &mut Model,
    msg: Message,
) -> Result<Option<Message>> {
    match msg {
        Message::Quit => {
            model.state = State::Quit;
        }
        Message::SelectNextNode => {
            model.select_next_node();
        }
        Message::SelectPrevNode => {
            model.select_prev_node();
        }
        Message::SelectParentNode => {
            model.select_parent_node()?;
        }
        Message::SelectNextSiblingNode => {
            model.select_current_next_sibling_node()?;
        }
        Message::SelectPrevSiblingNode => {
            model.select_current_prev_sibling_node()?;
        }
        Message::ToggleLogListFold => {
            model.toggle_current_fold()?;
        }
        Message::ScrollDown => {
            model.scroll_down_once();
        }
        Message::ScrollUp => {
            model.scroll_up_once();
        }
        Message::ScrollDownPage => {
            model.scroll_down_lines(model.log_list_height);
        }
        Message::ScrollUpPage => {
            model.scroll_up_lines(model.log_list_height);
        }
        Message::Refresh => {
            model.sync()?;
        }
        Message::Describe => {
            model.jj_describe(term)?;
        }
        Message::New => {
            model.jj_new()?;
        }
        Message::Abandon => {
            model.jj_abandon()?;
        }
        Message::Undo => {
            model.jj_undo()?;
        }
        Message::Commit => {
            model.jj_commit(term)?;
        }
        Message::Squash => {
            model.jj_squash(term)?;
        }
        Message::Edit => {
            model.jj_edit()?;
        }
        Message::Fetch => {
            model.jj_fetch()?;
        }
        Message::Push => {
            model.jj_push()?;
        }
    };

    Ok(None)
}
