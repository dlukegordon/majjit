use std::str::FromStr;

use crate::model::Model;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, Paragraph},
};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let mut header_spans = vec![
        Span::styled("repository: ", Style::default().fg(Color::Blue)),
        Span::styled(
            &model.global_args.repository,
            Style::default().fg(Color::Green),
        ),
        Span::raw("  "),
        Span::styled("revset: ", Style::default().fg(Color::Blue)),
        Span::styled(&model.revset, Style::default().fg(Color::Green)),
    ];
    if model.global_args.ignore_immutable {
        header_spans.push(Span::styled(
            "  --ignore-immutable",
            Style::default().fg(Color::LightRed),
        ));
    }
    let header = Paragraph::new(Line::from(header_spans));

    let log_list = List::new(model.log_list.clone())
        .highlight_style(Style::new().bold().bg(Color::from_str("#282A36").unwrap()))
        .scroll_padding(model.log_list_scroll_padding);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            if model.info_list.is_some() {
                Constraint::Ratio(1, 4)
            } else {
                Constraint::Length(0)
            },
        ])
        .split(frame.area());

    frame.render_widget(header, layout[0]);
    frame.render_stateful_widget(log_list, layout[1], &mut model.log_list_state);
    model.log_list_layout = layout[1];

    if let Some(info_list) = &model.info_list {
        let info_list = List::new(info_list.clone()).block(
            Block::default()
                .borders(Borders::TOP)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue)),
        );
        frame.render_widget(info_list, layout[2]);
    }
}
