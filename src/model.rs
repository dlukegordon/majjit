use crate::{
    jj_commands,
    jj_log::{JjLog, TreePosition},
};

use anyhow::{Result, anyhow};
use ratatui::{Terminal, backend::Backend, text::Text, widgets::ListState};

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
        self.sync_log_list()?;
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

    pub fn describe(&mut self, list_idx: usize, term: &mut Terminal<impl Backend>) -> Result<()> {
        let tree_pos = self.get_tree_position(list_idx)?;
        let commit = match self.jj_log.get_tree_commit(&tree_pos) {
            // If the cursor isn't over a commit or its child nodes, nothing to do
            None => return Ok(()),
            Some(commit) => commit,
        };

        jj_commands::describe(&self.repository, &commit.change_id, term)?;

        self.sync()?;
        Ok(())
    }
}
