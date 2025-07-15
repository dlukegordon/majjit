use crate::model::Model;

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
    SelectCurrentWorkingCopy,
    SelectParentNode,
    SelectNextSiblingNode,
    SelectPrevSiblingNode,
    ToggleLogListFold,
    Clear,
    ScrollDown,
    ScrollUp,
    ScrollDownPage,
    ScrollUpPage,
    LeftMouseClick { row: u16, column: u16 },
    RightMouseClick { row: u16, column: u16 },
    Refresh,
    Show,
    Describe,
    New,
    Abandon,
    Undo,
    Commit,
    Squash,
    Edit,
    Fetch,
    Push,
    BookmarkSetMaster,
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
        KeyCode::Char('@') => Some(Message::SelectCurrentWorkingCopy),
        KeyCode::Char('K') => Some(Message::SelectParentNode),
        KeyCode::Tab => Some(Message::ToggleLogListFold),
        KeyCode::Esc => Some(Message::Clear),
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Message::Refresh)
        }
        KeyCode::Enter => Some(Message::Show),
        KeyCode::Char('d') => Some(Message::Describe),
        KeyCode::Char('n') => Some(Message::New),
        KeyCode::Char('a') => Some(Message::Abandon),
        KeyCode::Char('u') => Some(Message::Undo),
        KeyCode::Char('c') => Some(Message::Commit),
        KeyCode::Char('s') => Some(Message::Squash),
        KeyCode::Char('e') => Some(Message::Edit),
        KeyCode::Char('f') => Some(Message::Fetch),
        KeyCode::Char('p') => Some(Message::Push),
        KeyCode::Char('m') => Some(Message::BookmarkSetMaster),
        _ => None,
    }
}

fn handle_mouse(mouse: event::MouseEvent) -> Option<Message> {
    match mouse.kind {
        MouseEventKind::ScrollDown => Some(Message::ScrollDown),
        MouseEventKind::ScrollUp => Some(Message::ScrollUp),
        MouseEventKind::Down(event::MouseButton::Left) => Some(Message::LeftMouseClick {
            row: mouse.row,
            column: mouse.column,
        }),
        MouseEventKind::Down(event::MouseButton::Right) => Some(Message::RightMouseClick {
            row: mouse.row,
            column: mouse.column,
        }),
        _ => None,
    }
}

fn handle_msg(
    term: &mut Terminal<impl Backend>,
    model: &mut Model,
    msg: Message,
) -> Result<Option<Message>> {
    match msg {
        Message::Quit => model.quit(),
        Message::SelectNextNode => model.select_next_node(),
        Message::SelectPrevNode => model.select_prev_node(),
        Message::SelectCurrentWorkingCopy => model.select_current_working_copy(),
        Message::SelectParentNode => model.select_parent_node()?,
        Message::SelectNextSiblingNode => model.select_current_next_sibling_node()?,
        Message::SelectPrevSiblingNode => model.select_current_prev_sibling_node()?,
        Message::ToggleLogListFold => model.toggle_current_fold()?,
        Message::Clear => model.clear(),
        Message::ScrollDown => model.scroll_down_once(),
        Message::ScrollUp => model.scroll_up_once(),
        Message::ScrollDownPage => model.scroll_down_page(),
        Message::ScrollUpPage => model.scroll_up_page(),
        Message::LeftMouseClick { row, column } => model.handle_mouse_click(row, column),
        Message::RightMouseClick { row, column } => {
            model.handle_mouse_click(row, column);
            model.toggle_current_fold()?;
        }
        Message::Refresh => model.sync()?,
        Message::Show => model.jj_show(term)?,
        Message::Describe => model.jj_describe(term)?,
        Message::New => model.jj_new()?,
        Message::Abandon => model.jj_abandon()?,
        Message::Undo => model.jj_undo()?,
        Message::Commit => model.jj_commit(term)?,
        Message::Squash => model.jj_squash(term)?,
        Message::Edit => model.jj_edit()?,
        Message::Fetch => model.jj_fetch()?,
        Message::Push => model.jj_push()?,
        Message::BookmarkSetMaster => model.jj_bookmark_set_master()?,
    };

    Ok(None)
}
