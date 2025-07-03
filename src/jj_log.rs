use crate::jj_commands;
use ansi_to_tui::IntoText;
use anyhow::{Error, Result, anyhow, bail};
use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
};
use regex::Regex;
use std::fmt;

#[derive(Debug)]
pub struct JjLog {
    pub log_tree: Vec<CommitOrText>,
}

impl JjLog {
    pub fn new() -> Result<Self> {
        Ok(JjLog {
            log_tree: Vec::new(),
        })
    }

    pub fn load_log_tree(&mut self, repository: &str, revset: &str) -> Result<()> {
        self.log_tree = CommitOrText::load_all(repository, revset)?;
        Ok(())
    }

    pub fn flatten_log(&mut self) -> Result<(Vec<Text<'static>>, Vec<TreePosition>)> {
        let mut log_list = Vec::new();
        let mut log_list_tree_positions = Vec::new();

        for (commit_or_text_idx, commit_or_text) in self.log_tree.iter_mut().enumerate() {
            commit_or_text.flatten(
                vec![commit_or_text_idx],
                &mut log_list,
                &mut log_list_tree_positions,
            )?;
        }

        Ok((log_list, log_list_tree_positions))
    }

    pub fn get_tree_node(&mut self, tree_pos: &TreePosition) -> Result<&mut dyn LogTreeNode> {
        // Traverse to commit
        let commit_or_text = &mut self.log_tree[tree_pos[COMMIT_OR_TEXT_IDX]];
        let commit = match commit_or_text {
            CommitOrText::InfoText(info_text) => {
                return Ok(info_text);
            }
            CommitOrText::Commit(commit) => commit,
        };

        let file_diff_idx = if tree_pos.len() <= FILE_DIFF_IDX {
            return Ok(commit);
        } else {
            tree_pos[FILE_DIFF_IDX]
        };

        // Traverse to file diff
        if !commit.loaded {
            bail!("Trying to get unloaded file diffs for commit");
        }
        let file_diff = &mut commit.file_diffs[file_diff_idx];
        let diff_hunk_idx = if tree_pos.len() <= DIFF_HUNK_IDX {
            return Ok(file_diff);
        } else {
            tree_pos[DIFF_HUNK_IDX]
        };

        // Traverse to diff hunk
        if !file_diff.loaded {
            bail!("Trying to get unloaded diff hunks for file diff");
        }
        let diff_hunk = &mut file_diff.diff_hunks[diff_hunk_idx];
        let diff_hunk_line_idx = if tree_pos.len() <= DIFF_HUNK_LINE_IDX {
            return Ok(diff_hunk);
        } else {
            tree_pos[DIFF_HUNK_LINE_IDX]
        };

        // Traverse to diff hunk line
        let diff_hunk_line = &mut diff_hunk.diff_hunk_lines[diff_hunk_line_idx];
        Ok(diff_hunk_line)
    }

    pub fn get_tree_commit(&self, tree_pos: &TreePosition) -> Option<&Commit> {
        let commit_or_text = &self.log_tree[tree_pos[COMMIT_OR_TEXT_IDX]];
        match commit_or_text {
            CommitOrText::InfoText(_) => None,
            CommitOrText::Commit(commit) => Some(commit),
        }
    }

    pub fn get_tree_file_diff(&self, tree_pos: &TreePosition) -> Option<&FileDiff> {
        if tree_pos.len() <= FILE_DIFF_IDX {
            return None;
        }
        let commit = match self.get_tree_commit(tree_pos) {
            None => return None,
            Some(commit) => commit,
        };
        Some(&commit.file_diffs[tree_pos[FILE_DIFF_IDX]])
    }

    pub fn get_current_commit(&self) -> Option<&Commit> {
        // TODO: cache this instead of looping each time?
        self.log_tree.iter().find_map(|item| match item {
            CommitOrText::Commit(commit) if commit.current_working_copy => Some(commit),
            _ => None,
        })
    }

    pub fn toggle_fold(&mut self, tree_pos: &TreePosition) -> Result<usize> {
        let mut tree_pos = tree_pos.clone();
        tree_pos.truncate(DIFF_HUNK_IDX + 1);
        let node = self.get_tree_node(&tree_pos)?;
        node.toggle_fold()?;
        Ok(node.flat_log_idx())
    }
}

pub trait LogTreeNode {
    fn render(&self) -> Result<Text<'static>>;
    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()>;
    fn flat_log_idx(&self) -> usize;
    fn children(&self) -> Vec<&dyn LogTreeNode>;
    fn toggle_fold(&mut self) -> Result<()>;
}

pub type TreePosition = Vec<usize>;
pub const COMMIT_OR_TEXT_IDX: usize = 0;
pub const FILE_DIFF_IDX: usize = 1;
pub const DIFF_HUNK_IDX: usize = 2;
pub const DIFF_HUNK_LINE_IDX: usize = 3;

pub fn get_parent_tree_position(tree_pos: &TreePosition) -> Option<TreePosition> {
    let mut tree_pos = tree_pos.clone();
    if tree_pos.len() > 1 {
        tree_pos.pop();
        Some(tree_pos)
    } else {
        None
    }
}

#[derive(Debug)]
pub enum CommitOrText {
    Commit(Commit),
    InfoText(InfoText),
}

impl CommitOrText {
    fn load_all(repository: &str, revset: &str) -> Result<Vec<Self>> {
        let output = jj_commands::log(repository, revset)?;
        let mut lines = output.trim().lines();
        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8}).*$")?;

        let mut commits_or_texts = Vec::new();
        loop {
            let line1 = match lines.next() {
                None => break,
                Some(line) => line,
            };

            if let None = re.captures(&strip_ansi(&line1)) {
                commits_or_texts.push(Self::InfoText(InfoText::new(line1.to_string())));
                continue;
            };

            let line2 = match lines.next() {
                None => "",
                Some(line2) => line2,
            };
            commits_or_texts.push(Self::Commit(Commit::new(
                repository.to_string(),
                format!("{line1}\n{line2}"),
            )?));
        }

        Ok(commits_or_texts)
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        match self {
            CommitOrText::Commit(commit) => {
                commit.flatten(tree_pos, log_list, log_list_tree_positions)
            }
            CommitOrText::InfoText(info_text) => {
                info_text.flatten(tree_pos, log_list, log_list_tree_positions)
            }
        }
    }

    pub fn flat_log_idx(&self) -> usize {
        match self {
            CommitOrText::Commit(commit) => commit.flat_log_idx(),
            CommitOrText::InfoText(info_text) => info_text.flat_log_idx,
        }
    }
}

#[derive(Debug)]
pub struct Commit {
    repository: String,
    pub change_id: String,
    _commit_id: String,
    pub current_working_copy: bool,
    has_conflict: bool,
    symbol: String,
    line1_graph_chars: String,
    line2_graph_chars: String,
    pretty_line1: String,
    pretty_line2: String,
    graph_indent: String,
    unfolded: bool,
    loaded: bool,
    file_diffs: Vec<FileDiff>,
    pub flat_log_idx: usize,
}

impl Commit {
    fn new(repository: String, pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re_fields =
            Regex::new(r"^([ │]*)(.)\s+([k-z]{8})\s+.*\s+([a-f0-9]{8})\s*(\S*)\s*\n([ │├─╯]*)")?;
        let re_lines = Regex::new(r"^[ │]*\S+\s+(.*)\n[ │├─╯]*(.*)")?;

        let captures = re_fields
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse commit fields: {:?}", clean_string))?;
        let line1_graph_chars: String = captures
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse line 2 graph chars"))?
            .as_str()
            .into();
        let symbol = captures
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse commit symbol"))?
            .as_str()
            .into();
        let change_id = captures
            .get(3)
            .ok_or_else(|| anyhow!("Cannot parse commit change id"))?
            .as_str()
            .into();
        let commit_id = captures
            .get(4)
            .ok_or_else(|| anyhow!("Cannot parse commit id"))?
            .as_str()
            .into();
        let conflict_status: String = captures
            .get(5)
            .ok_or_else(|| anyhow!("Cannot parse conflict status"))?
            .as_str()
            .into();
        let line2_graph_chars: String = captures
            .get(6)
            .ok_or_else(|| anyhow!("Cannot parse line 2 graph chars"))?
            .as_str()
            .into();
        let mut graph_indent: String = line2_graph_chars
            .chars()
            .map(|c| match c {
                '│' | ' ' => c,
                '├' => '│',
                _ => ' ',
            })
            .collect();
        graph_indent.pop(); // Even out with our spacing

        let current_working_copy = symbol == "@";
        let has_conflict = conflict_status == "conflict";

        let captures = re_lines
            .captures(&pretty_string)
            .ok_or_else(|| anyhow!("Cannot parse commit lines: {:?}", pretty_string))?;
        let pretty_line1 = captures
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse commit line1"))?
            .as_str()
            .into();
        let pretty_line2 = captures
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse commit line2"))?
            .as_str()
            .into();

        let mut commit = Commit {
            repository,
            change_id,
            _commit_id: commit_id,
            current_working_copy,
            has_conflict,
            symbol,
            line1_graph_chars,
            line2_graph_chars,
            pretty_line1,
            pretty_line2,
            graph_indent,
            unfolded: false,
            loaded: false,
            file_diffs: Vec::new(),
            flat_log_idx: 0,
        };

        // Start unfolded for @ commit
        if commit.current_working_copy {
            commit.toggle_fold()?;
        }

        Ok(commit)
    }
}

impl LogTreeNode for Commit {
    fn render(&self) -> Result<Text<'static>> {
        let mut line1 = Line::from(vec![
            Span::raw(self.line1_graph_chars.clone()),
            Span::styled(
                self.symbol.clone(),
                if self.has_conflict {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default()
                },
            ),
            Span::raw(" "),
            fold_symbol(self.unfolded),
            Span::raw(" "),
        ]);
        line1.extend(self.pretty_line1.into_text()?.lines[0].spans.clone());
        let mut lines = vec![line1];
        if self.pretty_line2 != "" {
            let mut line2 = Line::from(vec![
                Span::raw(self.line2_graph_chars.clone()),
                Span::raw(" "),
            ]);
            line2.extend(self.pretty_line2.into_text()?.lines[0].spans.clone());
            lines.push(line2);
        };
        Ok(Text::from(lines))
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        self.flat_log_idx = log_list.len();
        log_list.push(self.render()?);
        log_list_tree_positions.push(tree_pos.clone());

        if !self.unfolded {
            return Ok(());
        }

        for (file_diff_idx, file_diff) in self.file_diffs.iter_mut().enumerate() {
            let mut new_pos = tree_pos.clone();
            new_pos.push(file_diff_idx);
            file_diff.flatten(new_pos, log_list, log_list_tree_positions)?;
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn children(&self) -> Vec<&dyn LogTreeNode> {
        self.file_diffs
            .iter()
            .map(|fd| fd as &dyn LogTreeNode)
            .collect()
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        if !self.unfolded {
            return Ok(());
        }

        if !self.loaded {
            let file_diffs =
                FileDiff::load_all(&self.repository, &self.change_id, &self.graph_indent)?;
            self.file_diffs = file_diffs;
            self.loaded = true;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct InfoText {
    pretty_string: String,
    flat_log_idx: usize,
}

impl InfoText {
    fn new(pretty_string: String) -> Self {
        Self {
            pretty_string,
            flat_log_idx: 0,
        }
    }
}

impl LogTreeNode for InfoText {
    fn render(&self) -> Result<Text<'static>> {
        Ok(self.pretty_string.into_text()?)
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        self.flat_log_idx = log_list.len();
        log_list.push(self.render()?);
        log_list_tree_positions.push(tree_pos.clone());
        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn children(&self) -> Vec<&dyn LogTreeNode> {
        Vec::new()
    }

    fn toggle_fold(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct FileDiff {
    repository: String,
    change_id: String,
    pub path: String,
    description: String,
    status: FileDiffStatus,
    graph_indent: String,
    unfolded: bool,
    loaded: bool,
    diff_hunks: Vec<DiffHunk>,
    flat_log_idx: usize,
}

impl FileDiff {
    pub fn new(
        repository: String,
        change_id: String,
        pretty_string: String,
        graph_indent: String,
    ) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^([MADR])\s+(.+)$").unwrap();

        let captures = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse file diff string: {clean_string}"))?;
        let status = captures
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse file diff status"))?
            .as_str()
            .parse::<FileDiffStatus>()?;
        let description: String = captures
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse file diff path"))?
            .as_str()
            .into();

        let path = match status {
            FileDiffStatus::Renamed => {
                let rename_regex = Regex::new(r"^(.+)\{(.+?)\s*=>\s*(.+?)\}$").unwrap();
                let captures = rename_regex
                    .captures(&description)
                    .ok_or_else(|| anyhow!("Cannot parse file diff rename paths: {description}"))?;
                let path_start = captures
                    .get(1)
                    .ok_or_else(|| anyhow!("Cannot parse file diff rename path start"))?
                    .as_str();
                let path_new_end = captures
                    .get(3)
                    .ok_or_else(|| anyhow!("Cannot parse file diff rename path new end"))?
                    .as_str();

                format!("{path_start}{path_new_end}")
            }
            _ => description.clone(),
        };

        Ok(Self {
            repository,
            change_id,
            path,
            description,
            status,
            graph_indent,
            unfolded: false,
            loaded: false,
            diff_hunks: Vec::new(),
            flat_log_idx: 0,
        })
    }

    fn load_all(repository: &str, change_id: &str, graph_indent: &str) -> Result<Vec<Self>> {
        let output = jj_commands::diff_summary(repository, change_id)?;
        let lines: Vec<&str> = output.trim().lines().collect();

        let mut file_diffs = Vec::new();
        for line in lines {
            file_diffs.push(Self::new(
                repository.to_string(),
                change_id.to_string(),
                line.to_string(),
                graph_indent.to_string(),
            )?);
        }

        Ok(file_diffs)
    }
}

impl LogTreeNode for FileDiff {
    fn render(&self) -> Result<Text<'static>> {
        let line = Line::from(vec![
            Span::raw(self.graph_indent.clone()),
            fold_symbol(self.unfolded),
            Span::raw(" "),
            Span::styled(
                format!("{}  {}", self.status, self.description),
                Style::default().fg(Color::LightBlue),
            ),
        ]);
        Ok(Text::from(line))
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        self.flat_log_idx = log_list.len();
        log_list.push(self.render()?);
        log_list_tree_positions.push(tree_pos.clone());

        if !self.unfolded {
            return Ok(());
        }

        for (diff_hunk_idx, diff_hunk) in self.diff_hunks.iter_mut().enumerate() {
            let mut new_pos = tree_pos.clone();
            new_pos.push(diff_hunk_idx);
            diff_hunk.flatten(new_pos, log_list, log_list_tree_positions)?;
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn children(&self) -> Vec<&dyn LogTreeNode> {
        self.diff_hunks
            .iter()
            .map(|dh| dh as &dyn LogTreeNode)
            .collect()
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        if !self.unfolded {
            return Ok(());
        }

        if !self.loaded {
            let diff_hunks = DiffHunk::load_all(
                &self.repository,
                &self.change_id,
                &self.path,
                &self.graph_indent,
            )?;
            self.diff_hunks = diff_hunks;
            self.loaded = true;
        }

        Ok(())
    }
}

#[derive(Debug)]
enum FileDiffStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
}

impl std::str::FromStr for FileDiffStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M" => Ok(FileDiffStatus::Modified),
            "A" => Ok(FileDiffStatus::Added),
            "D" => Ok(FileDiffStatus::Deleted),
            "R" => Ok(FileDiffStatus::Renamed),
            _ => Err(anyhow!("Unknown file diff status: {}", s)),
        }
    }
}

impl fmt::Display for FileDiffStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let word = match self {
            FileDiffStatus::Modified => "modified",
            FileDiffStatus::Added => "new file",
            FileDiffStatus::Deleted => "deleted ",
            FileDiffStatus::Renamed => "renamed ",
        };
        write!(f, "{}", word)
    }
}

#[derive(Debug)]
struct DiffHunk {
    clean_string: String,
    graph_indent: String,
    unfolded: bool,
    _old_start: u32,
    _old_count: u32,
    _new_start: u32,
    _new_count: u32,
    diff_hunk_lines: Vec<DiffHunkLine>,
    flat_log_idx: usize,
}

impl DiffHunk {
    fn new(
        pretty_string: String,
        graph_indent: String,
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
    ) -> Self {
        let clean_string = strip_ansi(&pretty_string);
        Self {
            clean_string,
            graph_indent,
            unfolded: true,
            _old_start: old_start,
            _old_count: old_count,
            _new_start: new_start,
            _new_count: new_count,
            diff_hunk_lines: Vec::new(),
            flat_log_idx: 0,
        }
    }

    fn load_all(
        repository: &str,
        change_id: &str,
        file: &str,
        graph_indent: &str,
    ) -> Result<Vec<Self>> {
        let output = jj_commands::diff_file(repository, change_id, file)?;
        let output_lines: Vec<&str> = output.trim().lines().collect();

        let hunk_regex = Regex::new(r"@@ -(\d+),(\d+) \+(\d+),(\d+) @@")?;
        let mut diff_hunks = Vec::new();
        let mut maybe_diff_hunk: Option<Self> = None;

        for line in output_lines {
            let clean_line = strip_ansi(line);
            let maybe_captures = hunk_regex.captures(&clean_line);

            match maybe_captures {
                Some(captures) => {
                    let old_start = captures
                        .get(1)
                        .ok_or_else(|| anyhow!("Cannot parse hunk old start"))?
                        .as_str()
                        .parse()?;
                    let old_count = captures
                        .get(2)
                        .ok_or_else(|| anyhow!("Cannot parse hunk old count"))?
                        .as_str()
                        .parse()?;
                    let new_start = captures
                        .get(3)
                        .ok_or_else(|| anyhow!("Cannot parse hunk new start"))?
                        .as_str()
                        .parse()?;
                    let new_count = captures
                        .get(4)
                        .ok_or_else(|| anyhow!("Cannot parse hunk new count"))?
                        .as_str()
                        .parse()?;

                    if let Some(diff_hunk) = maybe_diff_hunk {
                        diff_hunks.push(diff_hunk)
                    };

                    maybe_diff_hunk = Some(Self::new(
                        line.to_string(),
                        graph_indent.to_string(),
                        old_start,
                        old_count,
                        new_start,
                        new_count,
                    ))
                }
                None => {
                    if let Some(mut diff_hunk) = maybe_diff_hunk {
                        diff_hunk.diff_hunk_lines.push(DiffHunkLine::new(
                            line.to_string(),
                            graph_indent.to_string(),
                        ));
                        maybe_diff_hunk = Some(diff_hunk);
                    }
                }
            }
        }

        if let Some(mut diff_hunk) = maybe_diff_hunk.take() {
            // Visual divider between hunk diff and next item in log list
            diff_hunk.diff_hunk_lines.push(DiffHunkLine::new(
                "\x1b[35m~\x1b[0m".to_string(),
                graph_indent.to_string(),
            ));
            diff_hunks.push(diff_hunk)
        };

        Ok(diff_hunks)
    }
}

impl LogTreeNode for DiffHunk {
    fn render(&self) -> Result<Text<'static>> {
        let line = Line::from(vec![
            Span::raw(self.graph_indent.clone()),
            fold_symbol(self.unfolded),
            Span::raw(" "),
            Span::styled(
                self.clean_string.clone(),
                Style::default().fg(Color::Magenta),
            ),
        ]);
        Ok(Text::from(line))
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        self.flat_log_idx = log_list.len();
        log_list.push(self.render()?);
        log_list_tree_positions.push(tree_pos.clone());

        if !self.unfolded {
            return Ok(());
        }

        for (diff_hunk_line_idx, diff_hunk_line) in self.diff_hunk_lines.iter_mut().enumerate() {
            let mut new_pos = tree_pos.clone();
            new_pos.push(diff_hunk_line_idx);
            diff_hunk_line.flatten(new_pos, log_list, log_list_tree_positions)?;
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn children(&self) -> Vec<&dyn LogTreeNode> {
        self.diff_hunk_lines
            .iter()
            .map(|hl| hl as &dyn LogTreeNode)
            .collect()
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        Ok(())
    }
}

#[derive(Debug)]
struct DiffHunkLine {
    pretty_string: String,
    graph_indent: String,
    flat_log_idx: usize,
}

impl DiffHunkLine {
    fn new(pretty_string: String, graph_indent: String) -> Self {
        Self {
            pretty_string,
            graph_indent,
            flat_log_idx: 0,
        }
    }
}

impl LogTreeNode for DiffHunkLine {
    fn render(&self) -> Result<Text<'static>> {
        let clean_string = strip_ansi(&self.pretty_string);
        let mut line = Line::from(vec![Span::raw(self.graph_indent.clone()), Span::raw("  ")]);

        for span in self.pretty_string.into_text()?.lines[0].spans.clone() {
            let span = if clean_string.starts_with("+") || clean_string.starts_with("-") {
                let style = span.style.clone().bold();
                span.style(style)
            } else {
                span
            };
            line.spans.push(span);
        }

        Ok(Text::from(line))
    }

    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()> {
        self.flat_log_idx = log_list.len();
        log_list.push(self.render()?);
        log_list_tree_positions.push(tree_pos);
        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn children(&self) -> Vec<&dyn LogTreeNode> {
        Vec::new()
    }

    fn toggle_fold(&mut self) -> Result<()> {
        Ok(())
    }
}

fn strip_ansi(pretty_str: &str) -> String {
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(&pretty_str, "").to_string()
}

fn fold_symbol(unfolded: bool) -> Span<'static> {
    let symbol = if unfolded { "▾" } else { "▸" };
    Span::styled(symbol, Style::default().fg(Color::DarkGray))
}
