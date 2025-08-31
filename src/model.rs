use std::io::Stdout;

use crate::{
    command_tree::{CommandTree, CommandTreeNode, display_error_lines},
    jj_commands::{JjCommand, JjCommandError},
    log_tree::{DIFF_HUNK_LINE_IDX, JjLog, TreePosition, get_parent_tree_position},
    update::Message,
};
use ansi_to_tui::IntoText;
use anyhow::Result;
use crossterm::event::KeyCode;
use ratatui::{Terminal, layout::Rect, prelude::CrosstermBackend, text::Text, widgets::ListState};

const LOG_LIST_SCROLL_PADDING: usize = 0;

#[derive(Default, Debug, PartialEq, Eq)]
pub enum State {
    #[default]
    Running,
    Quit,
}

#[derive(Debug, Clone)]
pub struct GlobalArgs {
    pub repository: String,
    pub ignore_immutable: bool,
}

#[derive(Debug)]
pub struct Model {
    pub global_args: GlobalArgs,
    pub revset: String,
    pub state: State,
    pub command_tree: CommandTree,
    command_keys: Vec<KeyCode>,
    jj_log: JjLog,
    pub log_list: Vec<Text<'static>>,
    pub log_list_state: ListState,
    log_list_tree_positions: Vec<TreePosition>,
    pub log_list_layout: Rect,
    pub log_list_scroll_padding: usize,
    pub info_list: Option<Text<'static>>,
}

#[derive(Debug)]
enum ScrollDirection {
    Up,
    Down,
}

type Term = Terminal<CrosstermBackend<Stdout>>;

impl Model {
    pub fn new(repository: String, revset: String) -> Result<Self> {
        let mut model = Self {
            state: State::default(),
            command_tree: CommandTree::new(),
            command_keys: Vec::new(),
            jj_log: JjLog::new()?,
            log_list: Vec::new(),
            log_list_state: ListState::default(),
            log_list_tree_positions: Vec::new(),
            log_list_layout: Rect::ZERO,
            log_list_scroll_padding: LOG_LIST_SCROLL_PADDING,
            info_list: None,
            global_args: GlobalArgs {
                repository,
                ignore_immutable: false,
            },
            revset,
        };

        model.sync()?;
        Ok(model)
    }

    pub fn quit(&mut self) {
        self.state = State::Quit;
    }

    fn reset_log_list_selection(&mut self) -> Result<()> {
        // Start with @ selected and unfolded
        let list_idx = match self.jj_log.get_current_commit() {
            None => 0,
            Some(commit) => commit.flat_log_idx,
        };
        self.log_select(list_idx);
        self.toggle_current_fold()
    }

    pub fn sync(&mut self) -> Result<()> {
        self.jj_log.load_log_tree(&self.global_args, &self.revset)?;
        self.sync_log_list()?;
        self.reset_log_list_selection()?;
        Ok(())
    }

    fn sync_log_list(&mut self) -> Result<()> {
        (self.log_list, self.log_list_tree_positions) = self.jj_log.flatten_log()?;
        Ok(())
    }

    pub fn toggle_ignore_immutable(&mut self) {
        self.global_args.ignore_immutable = !self.global_args.ignore_immutable;
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

    fn get_selected_change_id(&self) -> Option<&str> {
        let tree_pos = self.get_selected_tree_position();
        match self.jj_log.get_tree_commit(&tree_pos) {
            None => None,
            Some(commit) => Some(&commit.change_id),
        }
    }

    fn get_selected_file_path(&self) -> Option<&str> {
        let tree_pos = self.get_selected_tree_position();
        match self.jj_log.get_tree_file_diff(&tree_pos) {
            None => None,
            Some(file_diff) => Some(&file_diff.path),
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
        let log_list_selected_idx = self.jj_log.toggle_fold(&self.global_args, &tree_pos)?;
        self.sync_log_list()?;
        self.log_select(log_list_selected_idx);
        Ok(())
    }

    pub fn clear(&mut self) {
        self.info_list = None;
        self.command_keys.clear();
    }

    pub fn show_help(&mut self) {
        self.info_list = Some(self.command_tree.get_help());
    }

    pub fn handle_command_key(&mut self, key_code: KeyCode) -> Option<Message> {
        self.command_keys.push(key_code);

        let node = match self.command_tree.get_node(&self.command_keys) {
            None => {
                self.command_keys.pop();
                display_error_lines(&mut self.info_list, &key_code);
                return None;
            }
            Some(node) => node,
        };
        match node {
            CommandTreeNode::Children(children) => {
                self.info_list = Some(children.get_help());
                None
            }
            CommandTreeNode::Action(message) => {
                self.command_keys.clear();
                Some(*message)
            }
        }
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

    pub fn jj_show(&mut self, term: &mut Term) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let maybe_file_path = self.get_selected_file_path();
        let cmd = JjCommand::show(change_id, maybe_file_path, self.global_args.clone(), term);
        self.run_jj_command_nosync(cmd)
    }

    pub fn jj_describe(&mut self, term: &mut Term) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::describe(change_id, self.global_args.clone(), term);
        self.run_jj_command(cmd)
    }

    pub fn jj_new(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::new(change_id, self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_new_before(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::new_before(change_id, self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_abandon(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::abandon(change_id, self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_undo(&mut self) -> Result<()> {
        let cmd = JjCommand::undo(self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_commit(&mut self, term: &mut Term) -> Result<()> {
        let cmd = JjCommand::commit(self.global_args.clone(), term);
        self.run_jj_command(cmd)
    }

    pub fn jj_squash(&mut self, term: &mut Term) -> Result<()> {
        let tree_pos = self.get_selected_tree_position();
        let Some(commit) = self.jj_log.get_tree_commit(&tree_pos) else {
            return Ok(());
        };
        let maybe_file_path = self.get_selected_file_path();

        let cmd = if commit.description_first_line.is_none() {
            JjCommand::squash_noninteractive(
                &commit.change_id,
                maybe_file_path,
                self.global_args.clone(),
            )
        } else {
            JjCommand::squash_interactive(
                &commit.change_id,
                maybe_file_path,
                self.global_args.clone(),
                term,
            )
        };
        self.run_jj_command(cmd)
    }

    pub fn jj_edit(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::edit(change_id, self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_fetch(&mut self) -> Result<()> {
        let cmd = JjCommand::fetch(self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_push(&mut self) -> Result<()> {
        let cmd = JjCommand::push(self.global_args.clone());
        self.run_jj_command(cmd)
    }

    pub fn jj_bookmark_set_master(&mut self) -> Result<()> {
        let Some(change_id) = self.get_selected_change_id() else {
            return Ok(());
        };
        let cmd = JjCommand::bookmark_set_master(change_id, self.global_args.clone());
        self.run_jj_command(cmd)
    }

    fn run_jj_command(&mut self, mut cmd: JjCommand) -> Result<()> {
        let result = cmd.run();
        self.handle_jj_command_result(result, true)
    }

    fn run_jj_command_nosync(&mut self, mut cmd: JjCommand) -> Result<()> {
        let result = cmd.run();
        self.handle_jj_command_result(result, false)
    }

    fn handle_jj_command_result(
        &mut self,
        result: Result<String, JjCommandError>,
        sync_on_success: bool,
    ) -> Result<()> {
        self.clear();

        match result {
            Ok(output) => {
                self.info_list = Some(output.into_text()?);
                if sync_on_success { self.sync() } else { Ok(()) }
            }
            Err(err) => match err {
                JjCommandError::Other { err } => Err(err),
                JjCommandError::Failed { stderr } => {
                    self.info_list = Some(stderr.into_text()?);
                    Ok(())
                }
            },
        }
    }
}
