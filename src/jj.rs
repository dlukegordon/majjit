use ansi_to_tui::IntoText;
use anyhow::{Error, Result, anyhow, bail};
use ratatui::text::Text;
use regex::Regex;
use std::{fmt, process::Command};

#[derive(Debug)]
pub struct Jj {
    _repository: String,
    revset: String,
    log: Vec<CommitOrText>,
}

impl Jj {
    pub fn new(repository: String, revset: String) -> Result<Self> {
        Self::ensure_valid_repo(&repository)?;
        let log = CommitOrText::load_all(&repository, &revset)?;
        let jj = Jj {
            _repository: repository,
            revset,
            log,
        };
        Ok(jj)
    }

    pub fn ensure_valid_repo(repository: &str) -> Result<()> {
        let output = Command::new("jj")
            .env("JJ_CONFIG", "/dev/null")
            .arg("--repository")
            .arg(repository)
            .arg("workspace")
            .arg("root")
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();
            let err_msg = stderr.strip_prefix("Error: ").unwrap_or(&stderr);
            bail!("{}", err_msg);
        }
    }

    fn run_jj_command(repository: &str, args: &[&str]) -> Result<String> {
        let mut command = Command::new("jj");
        command
            .env("JJ_CONFIG", "/dev/null")
            .arg("--color")
            .arg("always")
            .arg("--config")
            .arg("colors.'diff added token'={underline=false}")
            .arg("--config")
            .arg("colors.'diff removed token'={underline=false}")
            .arg("--config")
            .arg("colors.'diff token'={underline=false}")
            .arg("--repository")
            .arg(repository)
            .args(args);
        let output = command.output()?;

        if output.status.success() {
            let stdout = String::from_utf8(output.stdout)?;
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Jj command '{:?}' failed with status {}: {}",
                command,
                output.status,
                stderr
            );
        }
    }

    fn run_jj_log(repository: &str, revset: &str) -> Result<String> {
        let args = ["log", "--revisions", revset];
        Self::run_jj_command(repository, &args)
    }

    pub fn run_jj_diff_summary(repository: &str, change_id: &str) -> Result<String> {
        let args = ["diff", "--revisions", change_id, "--summary"];
        Self::run_jj_command(repository, &args)
    }

    pub fn run_jj_diff_file(repository: &str, change_id: &str, file: &str) -> Result<String> {
        let args = ["diff", "--revisions", change_id, "--git", file];
        Self::run_jj_command(repository, &args)
    }

    pub fn revset(&self) -> &str {
        &self.revset
    }

    pub fn flatten_log(&mut self) -> Result<(Vec<Text<'static>>, Vec<TreePosition>)> {
        let mut log_list = Vec::new();
        let mut log_list_tree_positions = Vec::new();

        for (commit_or_text_idx, commit_or_text) in self.log.iter_mut().enumerate() {
            commit_or_text.flatten(
                TreePosition::new(commit_or_text_idx, None, None, None),
                &mut log_list,
                &mut log_list_tree_positions,
            )?;
        }

        Ok((log_list, log_list_tree_positions))
    }

    fn get_tree_node(&mut self, tree_pos: &TreePosition) -> Result<&mut dyn LogTreeNode> {
        // Traverse to commit
        let commit_or_text = &mut self.log[tree_pos.commit_or_text_idx];
        let commit = match commit_or_text {
            CommitOrText::InfoText(info_text) => {
                return Ok(info_text);
            }
            CommitOrText::Commit(commit) => commit,
        };

        let file_diff_idx = match tree_pos.file_diff_idx {
            None => {
                return Ok(commit);
            }
            Some(file_diff_idx) => file_diff_idx,
        };

        // Traverse to file diff
        let file_diff = match &mut commit.file_diffs {
            None => {
                bail!("Trying to get unloaded file diffs for commit");
            }
            Some(file_diffs) => &mut file_diffs[file_diff_idx],
        };
        let diff_hunk_idx = match tree_pos.diff_hunk_idx {
            None => {
                return Ok(file_diff);
            }
            Some(diff_hunk_idx) => diff_hunk_idx,
        };

        // Traverse to diff hunk
        let diff_hunk = match &mut file_diff.diff_hunks {
            None => {
                bail!("Trying to get unloaded diff hunks for file diff");
            }
            Some(diff_hunks) => &mut diff_hunks[diff_hunk_idx],
        };
        let diff_hunk_line_idx = match tree_pos.diff_hunk_line_idx {
            None => {
                return Ok(diff_hunk);
            }
            Some(diff_hunk_line_idx) => diff_hunk_line_idx,
        };

        // Traverse to diff hunk line
        let diff_hunk_line = &mut diff_hunk.diff_hunk_lines[diff_hunk_line_idx];
        Ok(diff_hunk_line)
    }

    pub fn toggle_fold(&mut self, tree_pos: &TreePosition) -> Result<usize> {
        let mut tree_pos = tree_pos.clone();
        tree_pos.diff_hunk_line_idx = None;
        let node = self.get_tree_node(&tree_pos)?;
        node.toggle_fold()?;
        Ok(node.flat_log_idx())
    }
}

trait LogTreeNode {
    fn render(&self) -> Result<Text<'static>>;
    fn flatten(
        &mut self,
        tree_pos: TreePosition,
        log_list: &mut Vec<Text<'static>>,
        log_list_tree_positions: &mut Vec<TreePosition>,
    ) -> Result<()>;
    fn flat_log_idx(&self) -> usize;
    fn toggle_fold(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TreePosition {
    pub commit_or_text_idx: usize,
    pub file_diff_idx: Option<usize>,
    pub diff_hunk_idx: Option<usize>,
    pub diff_hunk_line_idx: Option<usize>,
}

impl TreePosition {
    pub fn new(
        commit_or_text_idx: usize,
        file_diff_idx: Option<usize>,
        diff_hunk_idx: Option<usize>,
        diff_hunk_line_idx: Option<usize>,
    ) -> Self {
        Self {
            commit_or_text_idx,
            file_diff_idx,
            diff_hunk_idx,
            diff_hunk_line_idx,
        }
    }
}

#[derive(Debug)]
enum CommitOrText {
    Commit(Commit),
    InfoText(InfoText),
}

impl CommitOrText {
    fn load_all(repository: &str, revset: &str) -> Result<Vec<Self>> {
        let output = Jj::run_jj_log(repository, revset)?;
        let mut lines = output.trim().lines();
        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8})$")?;

        let mut commits_or_texts = Vec::new();
        loop {
            let line1 = match lines.next() {
                None => break,
                Some(line) => line,
            };

            if let None = re.captures(&strip_ansi(&line1)) {
                commits_or_texts.push(Self::InfoText(InfoText::new(line1.to_string())));
                break;
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
}

#[derive(Debug)]
struct Commit {
    repository: String,
    change_id: String,
    _commit_id: String,
    current_working_copy: bool,
    symbol: String,
    line1_graph_chars: String,
    line2_graph_chars: String,
    pretty_line1: String,
    pretty_line2: String,
    graph_indent: String,
    unfolded: bool,
    file_diffs: Option<Vec<FileDiff>>,
    flat_log_idx: usize,
}

impl Commit {
    fn new(repository: String, pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re_fields = Regex::new(r"^([ │]*)(.)\s+([k-z]{8})\s+.*\s+([a-f0-9]{8})\n([ │├─╯]*)")?;
        let re_lines = Regex::new(r"^[ │]*\S+\s+(.*)\n[ │├─╯]*(.*)")?;

        let captures = re_fields
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse commit fields: {:?}", pretty_string))?;
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
        let line2_graph_chars: String = captures
            .get(5)
            .ok_or_else(|| anyhow!("Cannot parse line 2 graph chars"))?
            .as_str()
            .into();
        let graph_indent: String = line2_graph_chars
            .chars()
            .map(|c| match c {
                '│' | ' ' => c,
                '├' => '│',
                _ => ' ',
            })
            .collect();

        let current_working_copy = symbol == "@";

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
            symbol,
            line1_graph_chars,
            line2_graph_chars,
            pretty_line1,
            pretty_line2,
            graph_indent,
            unfolded: false,
            file_diffs: None,
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
        let line1 = format!(
            "{}{}  {} {}",
            self.line1_graph_chars,
            self.symbol,
            fold_symbol(self.unfolded),
            self.pretty_line1
        );
        let line2 = if self.pretty_line2 == "" {
            "".to_string()
        } else {
            format!("\n{}  {}", self.line2_graph_chars, self.pretty_line2)
        };

        Ok(format!("{}{}", line1, line2).into_text()?)
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

        if let Some(file_diffs) = &mut self.file_diffs {
            for (file_diff_idx, file_diff) in file_diffs.iter_mut().enumerate() {
                file_diff.flatten(
                    TreePosition::new(tree_pos.commit_or_text_idx, Some(file_diff_idx), None, None),
                    log_list,
                    log_list_tree_positions,
                )?;
            }
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        if !self.unfolded {
            return Ok(());
        }

        if let None = self.file_diffs {
            let file_diffs =
                FileDiff::load_all(&self.repository, &self.change_id, &self.graph_indent)?;
            self.file_diffs = Some(file_diffs);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct InfoText {
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

    fn toggle_fold(&mut self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct FileDiff {
    repository: String,
    change_id: String,
    path: String,
    description: String,
    status: FileDiffStatus,
    graph_indent: String,
    unfolded: bool,
    diff_hunks: Option<Vec<DiffHunk>>,
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
            diff_hunks: None,
            flat_log_idx: 0,
        })
    }

    fn load_all(repository: &str, change_id: &str, graph_indent: &str) -> Result<Vec<Self>> {
        let output = Jj::run_jj_diff_summary(repository, change_id)?;
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
        Ok(format!(
            "{}{} {}  {}",
            self.graph_indent,
            fold_symbol(self.unfolded),
            self.status,
            self.description,
        )
        .into_text()?)
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

        if let Some(diff_hunks) = &mut self.diff_hunks {
            for (diff_hunk_idx, diff_hunk) in diff_hunks.iter_mut().enumerate() {
                diff_hunk.flatten(
                    TreePosition::new(
                        tree_pos.commit_or_text_idx,
                        tree_pos.file_diff_idx,
                        Some(diff_hunk_idx),
                        None,
                    ),
                    log_list,
                    log_list_tree_positions,
                )?;
            }
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        if !self.unfolded {
            return Ok(());
        }

        if let None = self.diff_hunks {
            let diff_hunks = DiffHunk::load_all(
                &self.repository,
                &self.change_id,
                &self.path,
                &self.graph_indent,
            )?;
            self.diff_hunks = Some(diff_hunks);
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
    pretty_string: String,
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
        lines: Vec<String>,
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
    ) -> Self {
        let diff_hunk_lines = lines
            .into_iter()
            .map(|line| DiffHunkLine::new(line, graph_indent.clone()))
            .collect();
        Self {
            pretty_string,
            graph_indent,
            unfolded: true,
            _old_start: old_start,
            _old_count: old_count,
            _new_start: new_start,
            _new_count: new_count,
            diff_hunk_lines,
            flat_log_idx: 0,
        }
    }

    fn load_all(
        repository: &str,
        change_id: &str,
        file: &str,
        graph_indent: &str,
    ) -> Result<Vec<Self>> {
        let output = Jj::run_jj_diff_file(repository, change_id, file)?;
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
                        Vec::new(),
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

        if let Some(diff_hunk) = maybe_diff_hunk.take() {
            diff_hunks.push(diff_hunk)
        };

        Ok(diff_hunks)
    }
}

impl LogTreeNode for DiffHunk {
    fn render(&self) -> Result<Text<'static>> {
        Ok(format!(
            "{}{} {}",
            self.graph_indent,
            fold_symbol(self.unfolded),
            self.pretty_string
        )
        .into_text()?)
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
            diff_hunk_line.flatten(
                TreePosition::new(
                    tree_pos.commit_or_text_idx,
                    tree_pos.file_diff_idx,
                    tree_pos.diff_hunk_idx,
                    Some(diff_hunk_line_idx),
                ),
                log_list,
                log_list_tree_positions,
            )?;
        }

        Ok(())
    }

    fn flat_log_idx(&self) -> usize {
        self.flat_log_idx
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
        Ok(format!("{0}  {1}", self.graph_indent, self.pretty_string).into_text()?)
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

    fn toggle_fold(&mut self) -> Result<()> {
        Ok(())
    }
}

fn strip_ansi(pretty_str: &str) -> String {
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(&pretty_str, "").to_string()
}

fn fold_symbol(unfolded: bool) -> char {
    if unfolded { '▾' } else { '▸' }
}
