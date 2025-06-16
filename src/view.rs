use crate::model::Model;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{List, Paragraph},
};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Header paragraph
            Constraint::Min(0),    // Rest for the list
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled("revset: ", Style::default().fg(Color::Blue)),
        Span::styled(model.jj.revset(), Style::default().fg(Color::Green)),
    ]));

    // Create commit list
    let commit_list =
        List::new(model.log_list.clone()).highlight_style(Style::new().bold().bg(Color::Black));

    // Render both widgets
    frame.render_widget(header, chunks[0]);
    frame.render_stateful_widget(commit_list, chunks[1], &mut model.log_list_state);
}
