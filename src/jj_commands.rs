use anyhow::{Result, bail};
use ratatui::{Terminal, backend::Backend};
use std::process::{Command, ExitStatus};

use crate::terminal;

#[derive(Debug)]
pub struct JjCommandError {
    pub command: String,
    pub status: ExitStatus,
    pub stderr: String,
}

impl std::fmt::Display for JjCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Jj command '{}' failed with {}:\n{}",
            self.command, self.status, self.stderr
        )
    }
}

impl std::error::Error for JjCommandError {}

fn run_jj_command(repository: &str, args: &[&str]) -> Result<String, JjCommandError> {
    let mut command = get_jj_command(repository);
    command.args(args);
    let output = command.output().unwrap();

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).unwrap();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into();
        Err(JjCommandError {
            command: format!("{args:?}"),
            status: output.status,
            stderr,
        })
    }
}

fn run_jj_command_interactive(
    repository: &str,
    args: &[&str],
    term: &mut Terminal<impl Backend>,
) -> Result<()> {
    let mut command = get_jj_command(repository);
    command.args(args);

    terminal::relinquish_terminal()?;
    let status = command.status()?;
    terminal::takeover_terminal(term)?;

    if status.success() {
        Ok(())
    } else {
        bail!("Jj command '{:?}' failed with status {}", command, status,);
    }
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
        let err_msg = stderr.strip_prefix("Error: ").unwrap_or(stderr);
        bail!("{}", err_msg);
    }
}

fn get_jj_command(repository: &str) -> Command {
    let mut command = Command::new("jj");
    command
        // .env("JJ_CONFIG", "/dev/null")
        .arg("--color")
        .arg("always")
        .arg("--config")
        .arg("colors.'diff added token'={underline=false}")
        .arg("--config")
        .arg("colors.'diff removed token'={underline=false}")
        .arg("--config")
        .arg("colors.'diff token'={underline=false}")
        .arg("--config")
        .arg(
            r#"templates.log_node=
                    coalesce(
                      if(!self, label("elided", "~")),
                      label(
                        separate(" ",
                          if(current_working_copy, "working_copy"),
                          if(immutable, "immutable"),
                          if(conflict, "conflict"),
                        ),
                        coalesce(
                          if(current_working_copy, "@"),
                          if(root, "┴"),
                          if(immutable, "●"),
                          if(conflict, "⊗"),
                          "○",
                        )
                      )
                    )
                "#,
        )
        .arg("--repository")
        .arg(repository);
    // TODO: this should be toggleable
    // .arg("--ignore-immutable");

    command
}

pub fn log(repository: &str, revset: &str) -> Result<String, JjCommandError> {
    let args = ["log", "--revisions", revset];
    run_jj_command(repository, &args)
}

pub fn diff_summary(repository: &str, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["diff", "--revisions", change_id, "--summary"];
    run_jj_command(repository, &args)
}

pub fn diff_file(repository: &str, change_id: &str, file: &str) -> Result<String, JjCommandError> {
    let args = ["diff", "--revisions", change_id, "--git", file];
    run_jj_command(repository, &args)
}

pub fn describe(
    repository: &str,
    change_id: &str,
    terminal: &mut Terminal<impl Backend>,
) -> Result<()> {
    let args = ["describe", change_id];
    run_jj_command_interactive(repository, &args, terminal)?;
    Ok(())
}

pub fn new(repository: &str, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["new", change_id];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn abandon(repository: &str, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["abandon", change_id];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn undo(repository: &str) -> Result<(), JjCommandError> {
    let args = ["undo"];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn commit(repository: &str, term: &mut Terminal<impl Backend>) -> Result<()> {
    let args = ["commit"];
    run_jj_command_interactive(repository, &args, term)?;
    Ok(())
}

pub fn squash(
    repository: &str,
    change_id: &str,
    maybe_file_path: Option<&str>,
    term: &mut Terminal<impl Backend>,
) -> Result<()> {
    let mut args = vec!["squash", "--revision", change_id];
    if let Some(file_path) = maybe_file_path {
        args.push(file_path);
    }
    run_jj_command_interactive(repository, &args, term)?;
    Ok(())
}

pub fn edit(repository: &str, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["edit", change_id];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn fetch(repository: &str) -> Result<(), JjCommandError> {
    let args = ["git", "fetch"];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn push(repository: &str) -> Result<(), JjCommandError> {
    let args = ["git", "push"];
    run_jj_command(repository, &args)?;
    Ok(())
}

pub fn bookmark_set_master(repository: &str, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["bookmark", "set", "master", "--revision", change_id];
    run_jj_command(repository, &args)?;
    Ok(())
}
