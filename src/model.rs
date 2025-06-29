use crate::{
    jj_commands,
    jj_log::{DIFF_HUNK_LINE_IDX, JjLog, TreePosition, get_parent_tree_position},
};

use anyhow::Result;
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
    pub log_list_height: usize,
}

impl Model {
    pub fn new(repository: String, revset: String) -> Result<Self> {
        let mut model = Self {
            state: State::default(),
            jj_log: JjLog::new()?,
            log_list: Vec::new(),
            log_list_state: ListState::default(),
            log_list_tree_positions: Vec::new(),
            log_list_height: 0,
            repository,
            revset,
        };
        model.sync()?;

        Ok(model)
    }

    fn reset_log_list_state(&mut self) {
        let list_idx = match self.jj_log.get_current_commit() {
            None => 0,
            Some(commit) => commit.flat_log_idx,
        };
        self.log_select(list_idx);
    }

    pub fn sync(&mut self) -> Result<()> {
        self.jj_log.load_log_tree(&self.repository, &self.revset)?;
        self.sync_log_list()?;
        self.reset_log_list_state();
        Ok(())
    }

    fn sync_log_list(&mut self) -> Result<()> {
        (self.log_list, self.log_list_tree_positions) = self.jj_log.flatten_log()?;
        Ok(())
    }

    fn log_offset(&self) -> usize {
        self.log_list_state.offset()
    }

    fn log_selected(&self) -> usize {
        self.log_list_state.selected().unwrap()
    }

    fn log_select(&mut self, idx: usize) {
        self.log_list_state.select(Some(idx));
    }

    fn get_selected_tree_position(&self) -> TreePosition {
        self.log_list_tree_positions[self.log_selected()].clone()
    }

    fn get_selected_change_id(&self) -> Result<Option<&str>> {
        let tree_pos = self.get_selected_tree_position();
        match self.jj_log.get_tree_commit(&tree_pos) {
            None => Ok(None),
            Some(commit) => Ok(Some(&commit.change_id)),
        }
    }

    pub fn select_next_node(&mut self) {
        if self.log_list_state.selected().unwrap() < self.log_list.len() - 1 {
            self.log_list_state.select_next();
        }
    }

    pub fn select_prev_node(&mut self) {
        if self.log_list_state.selected().unwrap() > 0 {
            self.log_list_state.select_previous();
        }
    }

    pub fn select_parent_node(&mut self) -> Result<()> {
        let tree_pos = self.get_selected_tree_position();
        if let Some(parent_pos) = get_parent_tree_position(&tree_pos) {
            let parent_node_idx = self.jj_log.get_tree_node(&parent_pos)?.flat_log_idx();
            self.log_select(parent_node_idx);
        }
        Ok(())
    }

    pub fn select_current_next_sibling_node(&mut self) -> Result<()> {
        let tree_pos = self.get_selected_tree_position();
        self.select_next_sibling_node(tree_pos)
    }

    fn select_next_sibling_node(&mut self, tree_pos: TreePosition) -> Result<()> {
        let mut tree_pos = tree_pos;
        if tree_pos.len() == DIFF_HUNK_LINE_IDX + 1 {
            tree_pos = get_parent_tree_position(&tree_pos).unwrap();
        }
        let idx = tree_pos[tree_pos.len() - 1];

        match get_parent_tree_position(&tree_pos) {
            Some(parent_pos) => {
                let parent_node = self.jj_log.get_tree_node(&parent_pos)?;
                let children = parent_node.children();

                if idx == children.len() - 1 {
                    self.select_next_sibling_node(parent_pos)?;
                } else {
                    let sibling_idx = (idx + 1).min(children.len() - 1);
                    self.log_list_state
                        .select(Some(children[sibling_idx].flat_log_idx()));
                }
            }
            None => {
                let sibling_idx = (idx + 1).min(self.jj_log.log_tree.len() - 1);
                self.log_list_state
                    .select(Some(self.jj_log.log_tree[sibling_idx].flat_log_idx()));
            }
        };

        Ok(())
    }

    pub fn select_current_prev_sibling_node(&mut self) -> Result<()> {
        let tree_pos = self.get_selected_tree_position();
        self.select_prev_sibling_node(tree_pos)
    }

    fn select_prev_sibling_node(&mut self, tree_pos: TreePosition) -> Result<()> {
        if tree_pos.len() == DIFF_HUNK_LINE_IDX + 1 {
            let parent_pos = get_parent_tree_position(&tree_pos).unwrap();
            let parent_node_idx = self.jj_log.get_tree_node(&parent_pos)?.flat_log_idx();
            self.log_select(parent_node_idx);
            return Ok(());
        }
        let idx = tree_pos[tree_pos.len() - 1];

        match get_parent_tree_position(&tree_pos) {
            Some(parent_pos) => {
                let parent_node = self.jj_log.get_tree_node(&parent_pos)?;
                let children = parent_node.children();

                if idx == 0 {
                    let parent_node_idx = parent_node.flat_log_idx();
                    self.log_select(parent_node_idx);
                } else {
                    let sibling_idx = idx - 1;
                    self.log_list_state
                        .select(Some(children[sibling_idx].flat_log_idx()));
                }
            }
            None => {
                let sibling_idx = idx.saturating_sub(1);
                self.log_list_state
                    .select(Some(self.jj_log.log_tree[sibling_idx].flat_log_idx()));
            }
        };

        Ok(())
    }

    pub fn toggle_current_fold(&mut self) -> Result<()> {
        let tree_pos = self.get_selected_tree_position();
        let log_list_selected_idx = self.jj_log.toggle_fold(&tree_pos)?;
        self.sync_log_list()?;
        self.log_select(log_list_selected_idx);
        Ok(())
    }

    pub fn scroll_down_once(&mut self) {
        self.select_next_node();
        *self.log_list_state.offset_mut() = self.log_offset() + 1;
    }

    pub fn scroll_up_once(&mut self) {
        self.select_prev_node();
        *self.log_list_state.offset_mut() = self.log_offset().saturating_sub(1);
    }

    pub fn scroll_lines(&mut self, num_lines: usize, up: bool) {
        let original_log_selected = self.log_selected();
        let lines_from_offset = self.log_selected() - self.log_offset();
        // If we're at the top of the list, just select the first line
        if up && self.log_offset() == 0 && lines_from_offset <= num_lines {
            self.log_select(0);
            return;
        }

        // To properly count multi line nodes, we must start scrolling from the top of the window
        self.log_select(self.log_offset());
        let mut amount_scrolled = 0;

        loop {
            let lines_in_node = self.log_list[self.log_selected()].lines.len();
            amount_scrolled += lines_in_node;
            if amount_scrolled > num_lines {
                break;
            }
            // If this scroll would put us past the end of the list, then revert just select the
            // last line and don't scroll. We must still traverse and then revert so we can
            // properly count multi line nodes
            if !up && self.log_offset() >= self.log_list.len() - 1 {
                self.log_select(self.log_list.len() - 1);
                *self.log_list_state.offset_mut() = original_log_selected - lines_from_offset;
                return;
            }

            if up {
                self.scroll_up_once();
            } else {
                self.scroll_down_once();
            }
        }

        // Restore the cursor position
        let new_offset = self.log_offset() + lines_from_offset;
        self.log_select(new_offset);
    }

    pub fn scroll_down_lines(&mut self, num_lines: usize) {
        self.scroll_lines(num_lines, false);
    }

    pub fn scroll_up_lines(&mut self, num_lines: usize) {
        self.scroll_lines(num_lines, true);
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
