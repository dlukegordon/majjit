use anyhow::{Result, anyhow, bail, ensure};
use regex::Regex;
use std::process::Command;

pub struct Jj {
    repository: String,
    revisions: String,
    log: Vec<Commit>,
}

impl Jj {
    pub fn init(repository: String, revisions: String) -> Result<Self> {
        let mut jj = Jj {
            repository,
            revisions,
            log: vec![],
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
        let args = ["log", "--revisions", &self.revisions];
        self.run_jj_command(&args)
    }
}

#[derive(Debug)]
struct Commit {
    change_id: String,
    commit_id: String,
    pretty_string: String,
}

impl Commit {
    fn new(pretty_string: String) -> Result<Self> {
        let ansi_regex = Regex::new(r"\x1b\[[0-9;]*m")?;
        let clean_string = ansi_regex.replace_all(&pretty_string, "").to_string();

        let re = Regex::new(r"^.+([k-z]{8})\s+.*\s+([a-f0-9]{8})\n")?;
        let caps = re
            .captures(&clean_string)
            .ok_or_else(|| anyhow!("Cannot parse commit string"))?;
        let change_id = caps
            .get(1)
            .ok_or_else(|| anyhow!("Cannot parse change id"))?
            .as_str()
            .into();
        let commit_id = caps
            .get(2)
            .ok_or_else(|| anyhow!("Cannot parse commit id"))?
            .as_str()
            .into();

        Ok(Commit {
            change_id,
            commit_id,
            pretty_string,
        })
    }
}
