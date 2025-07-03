use std::str::FromStr;

use crate::model::Model;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{List, Paragraph},
};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled("revset: ", Style::default().fg(Color::Blue)),
        Span::styled(&model.revset, Style::default().fg(Color::Green)),
    ]));

    let log_list = List::new(model.log_list.clone())
        .highlight_style(Style::new().bold().bg(Color::from_str("#282A36").unwrap()))
        .scroll_padding(model.log_list_scroll_padding);

    frame.render_widget(header, layout[0]);
    frame.render_stateful_widget(log_list, layout[1], &mut model.log_list_state);

    model.log_list_layout = layout[1];
}
