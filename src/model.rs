use crate::{
    jj_commands,
    jj_log::{JjLog, TreePosition},
};

use anyhow::{Result, anyhow, bail};
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
        let log_list_state = Self::default_log_list_state();

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

    fn default_log_list_state() -> ListState {
        let mut log_list_state = ListState::default();
        log_list_state.select(Some(0));
        log_list_state
    }

    pub fn sync(&mut self) -> Result<()> {
        self.jj_log.load_log_tree(&self.repository, &self.revset)?;
        self.sync_log_list()?;
        self.log_list_state = Self::default_log_list_state();
        Ok(())
    }

    fn sync_log_list(&mut self) -> Result<()> {
        (self.log_list, self.log_list_tree_positions) = self.jj_log.flatten_log()?;
        Ok(())
    }

    fn get_tree_position(&self) -> Result<TreePosition> {
        let list_idx = match self.log_list_state.selected() {
            None => bail!("No log list item is selected"),
            Some(list_idx) => list_idx,
        };
        self.log_list_tree_positions
            .get(list_idx)
            .cloned()
            .ok_or_else(|| anyhow!("Cannot get tree position for log list index {list_idx}"))
    }

    fn get_selected_change_id(&self) -> Result<Option<&str>> {
        let tree_pos = self.get_tree_position()?;
        match self.jj_log.get_tree_commit(&tree_pos) {
            None => Ok(None),
            Some(commit) => Ok(Some(&commit.change_id)),
        }
    }

    pub fn select_next_log(&mut self) -> Result<()> {
        let list_idx = match self.log_list_state.selected() {
            None => bail!("No log list item is selected"),
            Some(list_idx) => list_idx,
        };

        let next = if list_idx >= self.log_list.len() - 1 {
            list_idx
        } else {
            list_idx + 1
        };
        self.log_list_state.select(Some(next));
        Ok(())
    }

    pub fn select_prev_log(&mut self) -> Result<()> {
        let list_idx = match self.log_list_state.selected() {
            None => bail!("No log list item is selected"),
            Some(list_idx) => list_idx,
        };

        let prev = if list_idx == 0 {
            list_idx
        } else {
            list_idx - 1
        };
        self.log_list_state.select(Some(prev));
        Ok(())
    }

    pub fn toggle_fold(&mut self) -> Result<()> {
        let tree_pos = self.get_tree_position()?;
        let log_list_selected_idx = self.jj_log.toggle_fold(&tree_pos)?;
        self.sync_log_list()?;
        self.log_list_state.select(Some(log_list_selected_idx));
        Ok(())
    }

    pub fn jj_describe(&mut self, term: &mut Terminal<impl Backend>) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::describe(&self.repository, change_id, term)?;

        self.sync()?;
        Ok(())
    }

    pub fn jj_new(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::new(&self.repository, change_id)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_abandon(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::abandon(&self.repository, change_id)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_undo(&mut self) -> Result<()> {
        jj_commands::undo(&self.repository)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_commit(&mut self, term: &mut Terminal<impl Backend>) -> Result<()> {
        jj_commands::commit(&self.repository, term)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_squash(&mut self, term: &mut Terminal<impl Backend>) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::squash(&self.repository, change_id, term)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_edit(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::edit(&self.repository, change_id)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_fetch(&mut self) -> Result<()> {
        jj_commands::fetch(&self.repository)?;
        self.sync()?;
        Ok(())
    }

    pub fn jj_push(&mut self) -> Result<()> {
        jj_commands::push(&self.repository)?;
        self.sync()?;
        Ok(())
    }
}
