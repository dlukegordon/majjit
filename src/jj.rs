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

fn get_file_diffs(repository: &str, change_id: &str, indent: &str) -> Result<Vec<FileDiff>> {
    let output = run_jj_diff_summary(repository, change_id)?;
    let lines: Vec<&str> = output.trim().lines().collect();

    let mut file_diffs = Vec::new();
    for line in lines {
        file_diffs.push(FileDiff::new(line.to_string(), indent)?);
    }

    Ok(file_diffs)
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

    pub fn render_log(&self) -> Result<(Vec<Text<'static>>, Vec<TreePosition>)> {
        let mut log_list = Vec::new();
        let mut log_list_tree_positions = Vec::new();

        for (commit_idx, commit) in self.log.iter().enumerate() {
            log_list.push(commit.pretty_string.into_text()?);
            log_list_tree_positions.push(TreePosition {
                commit_idx,
                file_diff_idx: None,
            });

            if !commit.unfolded {
                continue;
            }

            if let Some(file_diffs) = &commit.file_diffs {
                for (file_diff_idx, file_diff) in file_diffs.iter().enumerate() {
                    log_list.push(file_diff.pretty_string.into_text()?);
                    log_list_tree_positions.push(TreePosition {
                        commit_idx,
                        file_diff_idx: Some(file_diff_idx),
                    });
                }
            }
        }

        Ok((log_list, log_list_tree_positions))
    }

    pub fn toggle_fold(&mut self, tree_pos: &TreePosition) -> Result<()> {
        let commit_idx = tree_pos.commit_idx;
        if commit_idx >= self.log.len() {
            bail!("Cannot get commit for tree position {tree_pos:?}");
        }

        if let Some(_) = tree_pos.file_diff_idx {
            bail!("Folding file diffs is not implemented yet");
        };

        self.log[commit_idx].toggle_fold()?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct TreePosition {
    pub commit_idx: usize,
    pub file_diff_idx: Option<usize>,
}

#[derive(Debug)]
struct Commit {
    change_id: String,
    commit_id: String,
    repository: String,
    pretty_string: String,
    indent: String,
    unfolded: bool,
    file_diffs: Option<Vec<FileDiff>>,
}

impl Commit {
    fn new(repository: String, pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8})\n([ │├─╯]*)")?;

        let caps = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse commit string"))?;
        let change_id = caps
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse commit change id"))?
            .as_str()
            .into();
        let commit_id = caps
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse commit id"))?
            .as_str()
            .into();

        let graph_chars: String = caps
            .get(3)
            .ok_or_else(|| anyhow!("Cannot parse indent prefix"))?
            .as_str()
            .into();
        let indent: String = (graph_chars + " ")
            .chars()
            .map(|c| match c {
                '│' | ' ' => c,
                '├' => '│',
                _ => ' ',
            })
            .collect();

        Ok(Self {
            change_id,
            commit_id,
            repository,
            pretty_string,
            indent,
            unfolded: false,
            file_diffs: None,
        })
    }

    fn toggle_fold(&mut self) -> Result<()> {
        self.unfolded = !self.unfolded;
        if !self.unfolded {
            return Ok(());
        }

        if let None = self.file_diffs {
            let file_diffs = get_file_diffs(&self.repository, &self.change_id, &self.indent)?;
            self.file_diffs = Some(file_diffs);
        }

        Ok(())
    }
}

impl TryFrom<&Commit> for Text<'_> {
    type Error = ansi_to_tui::Error;

    fn try_from(commit: &Commit) -> Result<Self, Self::Error> {
        commit.pretty_string.into_text()
    }
}

#[derive(Debug)]
struct FileDiff {
    path: String,
    status: FileDiffStatus,
    pretty_string: String,
}

impl FileDiff {
    pub fn new(pretty_string: String, indent: &str) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^([MAD])\s+(.+)$").unwrap();

        let caps = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse file diff string: {clean_string}"))?;
        let status = caps
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse file diff status"))?
            .as_str()
            .parse::<FileDiffStatus>()?;
        let path = caps
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse file diff path"))?
            .as_str()
            .into();

        let pretty_string = format!("{indent}{pretty_string}");

        Ok(Self {
            path,
            status,
            pretty_string,
        })
    }
}

impl TryFrom<&FileDiff> for Text<'_> {
    type Error = ansi_to_tui::Error;

    fn try_from(file_diff: &FileDiff) -> Result<Self, Self::Error> {
        file_diff.pretty_string.into_text()
    }
}

#[derive(Debug)]
enum FileDiffStatus {
    Modified,
    Added,
    Deleted,
}

impl std::str::FromStr for FileDiffStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "M" => Ok(FileDiffStatus::Modified),
            "A" => Ok(FileDiffStatus::Added),
            "D" => Ok(FileDiffStatus::Deleted),
            _ => Err(anyhow!("Unknown file diff status: {}", s)),
        }
    }
}

fn strip_ansi(pretty_str: &str) -> String {
    let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(&pretty_str, "").to_string()
}
