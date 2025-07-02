use crate::{
    jj_commands,
    jj_log::{DIFF_HUNK_LINE_IDX, JjLog, TreePosition, get_parent_tree_position},
};

use anyhow::Result;
use ratatui::{Terminal, backend::Backend, layout::Rect, text::Text, widgets::ListState};

const LOG_LIST_SCROLL_PADDING: usize = 0;

#[derive(Default, Debug, PartialEq, Eq)]
pub enum State {
    #[default]
    Running,
    Quit,
}

#[derive(Debug)]
pub struct Model {
    repository: String,
    pub revset: String,
    pub state: State,
    jj_log: JjLog,
    pub log_list: Vec<Text<'static>>,
    pub log_list_state: ListState,
    log_list_tree_positions: Vec<TreePosition>,
    pub log_list_layout: Rect,
    pub log_list_scroll_padding: usize,
}

#[derive(Debug)]
enum ScrollDirection {
    Up,
    Down,
}

impl Model {
    pub fn new(repository: String, revset: String) -> Result<Self> {
        let mut model = Self {
            state: State::default(),
            jj_log: JjLog::new()?,
            log_list: Vec::new(),
            log_list_state: ListState::default(),
            log_list_tree_positions: Vec::new(),
            log_list_layout: Rect::ZERO,
            log_list_scroll_padding: LOG_LIST_SCROLL_PADDING,
            repository,
            revset,
        };
        model.sync()?;

        Ok(model)
    }

    pub fn quit(&mut self) {
        self.state = State::Quit;
    }

    fn reset_log_list_selection(&mut self) {
        let list_idx = match self.jj_log.get_current_commit() {
            None => 0,
            Some(commit) => commit.flat_log_idx,
        };
        self.log_select(list_idx);
    }

    pub fn sync(&mut self) -> Result<()> {
        self.jj_log.load_log_tree(&self.repository, &self.revset)?;
        self.sync_log_list()?;
        self.reset_log_list_selection();
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

    pub fn select_current_working_copy(&mut self) {
        if let Some(commit) = self.jj_log.get_current_commit() {
            self.log_select(commit.flat_log_idx);
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
        if self.log_selected() <= self.log_offset() + self.log_list_scroll_padding {
            self.select_next_node();
        }
        *self.log_list_state.offset_mut() = self.log_offset() + 1;
    }

    pub fn scroll_up_once(&mut self) {
        if self.log_offset() == 0 {
            return;
        }
        let last_node_visible = self.line_dist_to_dest_node(
            self.log_list_layout.height as usize - 1,
            self.log_offset(),
            &ScrollDirection::Down,
        );
        if self.log_selected() >= last_node_visible - 1 - self.log_list_scroll_padding {
            self.select_prev_node();
        }
        *self.log_list_state.offset_mut() = self.log_offset().saturating_sub(1);
    }

    pub fn scroll_down_page(&mut self) {
        self.scroll_lines(self.log_list_layout.height as usize, &ScrollDirection::Down);
    }

    pub fn scroll_up_page(&mut self) {
        self.scroll_lines(self.log_list_layout.height as usize, &ScrollDirection::Up);
    }

    fn scroll_lines(&mut self, num_lines: usize, direction: &ScrollDirection) {
        let selected_node_dist_from_offset = self.log_selected() - self.log_offset();
        let mut target_offset =
            self.line_dist_to_dest_node(num_lines, self.log_offset(), direction);
        let mut target_node = target_offset + selected_node_dist_from_offset;
        match direction {
            ScrollDirection::Down => {
                if target_offset == self.log_list.len() - 1 {
                    target_node = target_offset;
                    target_offset = self.log_offset();
                }
            }
            ScrollDirection::Up => {
                // If we're already at the top of the page, then move selection to the top as well
                if target_offset == 0 && target_offset == self.log_offset() {
                    target_node = 0;
                }
            }
        }
        self.log_select(target_node);
        *self.log_list_state.offset_mut() = target_offset;
    }

    pub fn handle_mouse_click(&mut self, row: u16, column: u16) {
        let Rect {
            x,
            y,
            width,
            height,
        } = self.log_list_layout;

        // Check if inside log list
        if row < y || row >= y + height || column < x || column >= x + width {
            return;
        }

        let target_node = self.line_dist_to_dest_node(
            row as usize - y as usize,
            self.log_offset(),
            &ScrollDirection::Down,
        );
        self.log_select(target_node);
    }

    // Since some nodes contain multiple lines, we need a way to determine the destination node
    // which is n lines away from the starting node.
    fn line_dist_to_dest_node(
        &self,
        line_dist: usize,
        starting_node: usize,
        direction: &ScrollDirection,
    ) -> usize {
        let mut current_node = starting_node;
        let mut lines_traversed = 0;
        loop {
            let lines_in_node = self.log_list[current_node].lines.len();
            lines_traversed += lines_in_node;

            // Stop if we've found the dest node or have no further to traverse
            if match direction {
                ScrollDirection::Down => current_node == self.log_list.len() - 1,
                ScrollDirection::Up => current_node == 0,
            } || lines_traversed > line_dist
            {
                break;
            }

            match direction {
                ScrollDirection::Down => current_node += 1,
                ScrollDirection::Up => current_node -= 1,
            }
        }

        current_node
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

    pub fn jj_bookmark_set_master(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id()? else {
            return Ok(());
        };
        jj_commands::bookmark_set_master(&self.repository, change_id)?;
        self.sync()?;
        Ok(())
    }
}
