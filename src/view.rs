use crate::model::Model;

use ratatui::{Frame, widgets::Paragraph};

pub fn view(model: &mut Model, frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new(format!("Counter: {}", model.counter)),
        frame.area(),
    );
}
