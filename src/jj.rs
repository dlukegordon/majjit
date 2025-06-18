use ansi_to_tui::IntoText;
use anyhow::{Error, Result, anyhow, bail, ensure};
use ratatui::text::Text;
use regex::Regex;
use std::process::Command;

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
    run_jj_command(repository, &args)
}

pub fn run_jj_diff_summary(repository: &str, change_id: &str) -> Result<String> {
    let args = ["diff", "--revisions", change_id, "--summary"];
    run_jj_command(repository, &args)
}

pub fn run_jj_diff_file(repository: &str, change_id: &str, file: &str) -> Result<String> {
    let args = ["diff", "--revisions", change_id, "--git", file];
    run_jj_command(repository, &args)
}

fn get_commits(repository: &str, revset: &str) -> Result<Vec<Commit>> {
    let output = run_jj_log(repository, revset)?;
    let lines: Vec<&str> = output.trim().lines().collect();

    let mut commits = Vec::new();
    for chunk in lines.chunks(2) {
        match chunk {
            [line1, line2] => {
                commits.push(Commit::new(
                    repository.to_string(),
                    format!("{line1}\n{line2}"),
                )?);
            }
            [line1] => {
                ensure!(line1.contains("root()"), "Last line is not the root commit");
                commits.push(Commit::new(repository.to_string(), format!("{line1}\n"))?);
            }
            _ => bail!("Cannot parse log output"),
        }
    }

    Ok(commits)
}

fn get_file_diffs(repository: &str, change_id: &str, graph_indent: &str) -> Result<Vec<FileDiff>> {
    let output = run_jj_diff_summary(repository, change_id)?;
    let lines: Vec<&str> = output.trim().lines().collect();

    let mut file_diffs = Vec::new();
    for line in lines {
        file_diffs.push(FileDiff::new(
            repository.to_string(),
            change_id.to_string(),
            line.to_string(),
            graph_indent.to_string(),
        )?);
    }

    Ok(file_diffs)
}

fn get_file_diff_hunks(
    repository: &str,
    change_id: &str,
    file: &str,
    graph_indent: &str,
) -> Result<Vec<DiffHunk>> {
    let output = run_jj_diff_file(repository, change_id, file)?;
    let output_lines: Vec<&str> = output.trim().lines().collect();

    let hunk_regex = Regex::new(r"@@ -(\d+),(\d+) \+(\d+),(\d+) @@")?;
    let mut diff_hunks = Vec::new();
    let mut maybe_diff_hunk: Option<DiffHunk> = None;

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

                maybe_diff_hunk = Some(DiffHunk::new(
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

fn strip_ansi(pretty_str: &str) -> String {
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(&pretty_str, "").to_string()
}

#[derive(Debug)]
pub struct Jj {
    repository: String,
    revset: String,
    log: Vec<Commit>,
}

impl Jj {
    pub fn init(repository: String, revset: String) -> Result<Self> {
        let log = get_commits(&repository, &revset)?;
        let jj = Jj {
            repository,
            revset,
            log,
        };
        Ok(jj)
    }

    pub fn revset(&self) -> &str {
        &self.revset
    }

    pub fn flatten_log(&mut self) -> Result<(Vec<Text<'static>>, Vec<TreePosition>)> {
        let mut log_list = Vec::new();
        let mut log_list_tree_positions = Vec::new();

        for (commit_idx, commit) in self.log.iter_mut().enumerate() {
            commit.flatten(
                TreePosition::new(commit_idx, None, None, None),
                &mut log_list,
                &mut log_list_tree_positions,
            )?;
        }

        Ok((log_list, log_list_tree_positions))
    }

    pub fn get_tree_node(&mut self, tree_pos: &TreePosition) -> Result<&mut dyn LogTreeNode> {
        // Traverse to commit
        let commit = &mut self.log[tree_pos.commit_idx];
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
    pub commit_idx: usize,
    pub file_diff_idx: Option<usize>,
    pub diff_hunk_idx: Option<usize>,
    pub diff_hunk_line_idx: Option<usize>,
}

impl TreePosition {
    pub fn new(
        commit_idx: usize,
        file_diff_idx: Option<usize>,
        diff_hunk_idx: Option<usize>,
        diff_hunk_line_idx: Option<usize>,
    ) -> Self {
        Self {
            commit_idx,
            file_diff_idx,
            diff_hunk_idx,
            diff_hunk_line_idx,
        }
    }
}

#[derive(Debug)]
struct Commit {
    repository: String,
    change_id: String,
    commit_id: String,
    current_working_copy: bool,
    pretty_string: String,
    graph_indent: String,
    unfolded: bool,
    file_diffs: Option<Vec<FileDiff>>,
    flat_log_idx: usize,
}

impl Commit {
    fn new(repository: String, pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8})\n([ │├─╯]*)")?;

        let captures = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse commit string"))?;
        let change_id = captures
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse commit change id"))?
            .as_str()
            .into();
        let commit_id = captures
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse commit id"))?
            .as_str()
            .into();

        let graph_chars: String = captures
            .get(3)
            .ok_or_else(|| anyhow!("Cannot parse indent prefix"))?
            .as_str()
            .into();
        let graph_indent: String = graph_chars
            .chars()
            .map(|c| match c {
                '│' | ' ' => c,
                '├' => '│',
                _ => ' ',
            })
            .collect();

        let current_working_copy = clean_string.starts_with('@');

        let mut commit = Self {
            repository,
            change_id,
            commit_id,
            current_working_copy,
            pretty_string,
            graph_indent,
            unfolded: false,
            file_diffs: None,
            flat_log_idx: 0,
        };

        // Start unfolded for @ commit
        if current_working_copy {
            commit.toggle_fold()?;
        }

        Ok(commit)
    }
}

impl LogTreeNode for Commit {
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

        if !self.unfolded {
            return Ok(());
        }

        if let Some(file_diffs) = &mut self.file_diffs {
            for (file_diff_idx, file_diff) in file_diffs.iter_mut().enumerate() {
                file_diff.flatten(
                    TreePosition::new(tree_pos.commit_idx, Some(file_diff_idx), None, None),
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
            let file_diffs = get_file_diffs(&self.repository, &self.change_id, &self.graph_indent)?;
            self.file_diffs = Some(file_diffs);
        }

        Ok(())
    }
}

#[derive(Debug)]
struct FileDiff {
    repository: String,
    change_id: String,
    path: String,
    status: FileDiffStatus,
    pretty_string: String,
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
        let path: String = captures
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse file diff path"))?
            .as_str()
            .into();

        let path = match status {
            FileDiffStatus::Renamed => {
                let rename_regex = Regex::new(r"^(.+)\{(.+?)\s*=>\s*(.+?)\}$").unwrap();
                let captures = rename_regex
                    .captures(&path)
                    .ok_or_else(|| anyhow!("Cannot parse file diff rename paths: {path}"))?;
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
            _ => path,
        };

        Ok(Self {
            repository,
            change_id,
            path,
            status,
            pretty_string,
            graph_indent,
            unfolded: false,
            diff_hunks: None,
            flat_log_idx: 0,
        })
    }
}

impl LogTreeNode for FileDiff {
    fn render(&self) -> Result<Text<'static>> {
        Ok(format!("{0} {1}", self.graph_indent, self.pretty_string).into_text()?)
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
                        tree_pos.commit_idx,
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
            let diff_hunks = get_file_diff_hunks(
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

#[derive(Debug)]
struct DiffHunk {
    pretty_string: String,
    graph_indent: String,
    unfolded: bool,
    old_start: u32,
    old_count: u32,
    new_start: u32,
    new_count: u32,
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
            old_start,
            old_count,
            new_start,
            new_count,
            diff_hunk_lines,
            flat_log_idx: 0,
        }
    }
}

impl LogTreeNode for DiffHunk {
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
        log_list_tree_positions.push(tree_pos.clone());

        if !self.unfolded {
            return Ok(());
        }

        for (diff_hunk_line_idx, diff_hunk_line) in self.diff_hunk_lines.iter_mut().enumerate() {
            diff_hunk_line.flatten(
                TreePosition::new(
                    tree_pos.commit_idx,
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
        Ok(format!("{0}   {1}", self.graph_indent, self.pretty_string).into_text()?)
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
