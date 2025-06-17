use crate::jj::{Jj, TreePosition};

use anyhow::{Result, anyhow};
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
    pub log_list_tree_positions: Vec<TreePosition>,
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
            log_list_tree_positions: Vec::new(),
        };

        model.update_log_list()?;

        Ok(model)
    }

    fn update_log_list(&mut self) -> Result<()> {
        (self.log_list, self.log_list_tree_positions) = self.jj.flatten_log()?;
        Ok(())
    }

    pub fn toggle_fold(&mut self, list_idx: usize) -> Result<()> {
        let tree_pos = self
            .log_list_tree_positions
            .get(list_idx)
            .ok_or_else(|| anyhow!("Cannot get tree position for lost list index {list_idx}"))?;

        self.jj.toggle_fold(tree_pos)?;
        self.update_log_list()?;

        Ok(())
    }
}
