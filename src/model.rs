use crate::jj::Jj;

use anyhow::Result;
use ratatui::{text::Text, widgets::ListState};

#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Quit,
}

#[derive(Debug)]
pub struct Model {
    pub state: State,
    pub jj: Jj,
    pub log_list: Vec<Text<'static>>,
    pub log_list_state: ListState,
}

impl Model {
    pub fn new(jj: Jj) -> Result<Self> {
        let mut log_list_state = ListState::default();
        log_list_state.select(Some(0));

        let mut model = Self {
            state: State::Running,
            jj,
            log_list: Vec::new(),
            log_list_state,
        };
        model.update_log_list()?;

        Ok(model)
    }

    fn update_log_list(&mut self) -> Result<()> {
        self.log_list = self.jj.get_text_vec()?;
        Ok(())
    }
}
