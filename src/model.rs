use crate::jj_log::{JjLog, TreePosition};

use anyhow::{Result, anyhow};
use ratatui::{text::Text, widgets::ListState};

#[derive(Default, Debug, PartialEq, Eq)]
pub enum State {
    #[default]
    Running,
    Quit,
}

#[derive(Debug)]
pub struct Model {
    pub repository: String,
    pub revset: String,
    pub state: State,
    pub jj_log: JjLog,
    pub log_list: Vec<Text<'static>>,
    pub log_list_state: ListState,
    pub log_list_tree_positions: Vec<TreePosition>,
}

impl Model {
    pub fn new(repository: String, revset: String) -> Result<Self> {
        let jj_log = JjLog::new(&repository, &revset)?;
        let mut log_list_state = ListState::default();
        log_list_state.select(Some(0));

        let mut model = Self {
            state: State::default(),
            repository,
            revset,
            jj_log,
            log_list: Vec::new(),
            log_list_state,
            log_list_tree_positions: Vec::new(),
        };
        model.sync_log_list()?;

        Ok(model)
    }

    pub fn sync(&mut self) -> Result<()> {
        self.jj_log.load_log_tree(&self.repository, &self.revset)?;
        (self.log_list, self.log_list_tree_positions) = self.jj_log.flatten_log()?;
        Ok(())
    }

    fn sync_log_list(&mut self) -> Result<()> {
        (self.log_list, self.log_list_tree_positions) = self.jj_log.flatten_log()?;
        Ok(())
    }

    fn get_tree_position(&self, list_idx: usize) -> Result<TreePosition> {
        self.log_list_tree_positions
            .get(list_idx)
            .cloned()
            .ok_or_else(|| anyhow!("Cannot get tree position for log list index {list_idx}"))
    }

    pub fn toggle_fold(&mut self, list_idx: usize) -> Result<()> {
        let tree_pos = self.get_tree_position(list_idx)?;
        let log_list_selected_idx = self.jj_log.toggle_fold(&tree_pos)?;
        self.log_list_state.select(Some(log_list_selected_idx));
        self.sync_log_list()?;
        Ok(())
    }
}
