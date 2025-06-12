use crate::model::Model;

use ratatui::{
    Frame,
    style::{Style, Stylize},
    widgets::List,
};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let commit_list = List::new(model.commits.clone())
        .highlight_style(Style::new().bold())
        .highlight_symbol(">");

    frame.render_stateful_widget(commit_list, frame.area(), &mut model.commit_list_state);
}
