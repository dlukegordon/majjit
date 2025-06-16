use ansi_to_tui::IntoText;
use anyhow::{Error, Result, anyhow, bail, ensure};
use ratatui::text::Text;
use regex::Regex;
use std::process::Command;

#[derive(Debug)]
pub struct Jj {
    repository: String,
    revset: String,
    log: Vec<Commit>,
}

impl Jj {
    pub fn init(repository: String, revset: String) -> Result<Self> {
        let mut jj = Jj {
            repository,
            revset,
            log: Vec::new(),
        };
        jj.load_log()?;
        Ok(jj)
    }

    fn load_log(&mut self) -> Result<()> {
        let output = self.run_jj_log()?;
        let lines: Vec<&str> = output.trim().lines().collect();

        let mut commits = Vec::new();
        for chunk in lines.chunks(2) {
            match chunk {
                [line1, line2] => {
                    commits.push(Commit::new(format!("{line1}\n{line2}"))?);
                }
                [line1] => {
                    ensure!(line1.contains("root()"), "Last line is not the root commit");
                    commits.push(Commit::new(format!("{line1}\n"))?);
                }
                _ => bail!("Cannot parse log output"),
            }
        }

        self.log = commits;
        Ok(())
    }

    fn run_jj_command(&self, args: &[&str]) -> Result<String> {
        let mut command = Command::new("jj");
        command
            .env("JJ_CONFIG", "/dev/null")
            .arg("--color")
            .arg("always")
            .arg("--repository")
            .arg(&self.repository)
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

    fn run_jj_log(&self) -> Result<String> {
        let args = ["log", "--revisions", &self.revset];
        self.run_jj_command(&args)
    }

    fn run_jj_diff_summary(&self, change_id: &str) -> Result<String> {
        let args = ["diff", "--revisions", change_id];
        self.run_jj_command(&args)
    }

    pub fn get_text_vec(&self) -> Result<Vec<Text<'static>>> {
        let mut result = Vec::new();
        for commit in &self.log {
            result.push(commit.pretty_string.into_text()?);
        }
        Ok(result)
    }

    pub fn revset(&self) -> &str {
        &self.revset
    }
}

#[derive(Debug)]
struct Commit {
    change_id: String,
    commit_id: String,
    pretty_string: String,
    unfolded: bool,
    file_diffs: Option<Vec<FileDiff>>,
}

impl Commit {
    fn new(pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8})\n")?;

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

        Ok(Self {
            change_id,
            commit_id,
            pretty_string,
            unfolded: false,
            file_diffs: None,
        })
    }

    fn load_file_diffs(&mut self, jj: &Jj) -> Result<()> {
        let output = jj.run_jj_diff_summary(&self.change_id)?;
        let lines: Vec<&str> = output.trim().lines().collect();

        let mut file_diffs = Vec::new();
        for line in lines {
            file_diffs.push(FileDiff::new(line.to_string())?);
        }

        self.file_diffs = Some(file_diffs);
        Ok(())
    }

    fn toggle_fold(&mut self, jj: &Jj) -> Result<()> {
        self.unfolded = !self.unfolded;

        if !self.unfolded {
            return Ok(());
        }

        if let None = self.file_diffs {
            self.load_file_diffs(jj)?;
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
    pub fn new(pretty_string: String) -> Result<Self> {
        let clean_string = strip_ansi(&pretty_string);
        let re = Regex::new(r"^([MAD])\s+(.+)$").unwrap();

        let caps = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse file diff string"))?;
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
