use crate::model::GlobalArgs;
use crate::terminal;
use anyhow::{Result, anyhow, bail};
use ratatui::{Terminal, backend::Backend};
use std::{
    io::Read,
    process::{Command, ExitStatus},
};

#[derive(Debug)]
pub enum JjCommandError {
    Failed {
        command: String,
        status: ExitStatus,
        stderr: String,
    },
    Other {
        err: anyhow::Error,
    },
}

impl std::fmt::Display for JjCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed {
                command,
                status,
                stderr,
            } => {
                write!(
                    f,
                    "Jj command '{}' failed with {}:\n{}",
                    command, status, stderr
                )
            }
            Self::Other { err } => err.fmt(f),
        }
    }
}

impl std::error::Error for JjCommandError {}

fn run_jj_command(global_args: &GlobalArgs, args: &[&str]) -> Result<String, JjCommandError> {
    let mut command = get_jj_command(global_args);
    command.args(args);
    let output = command
        .output()
        .map_err(|e| JjCommandError::Other { err: e.into() })?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| JjCommandError::Other { err: e.into() })?;
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into();
        Err(JjCommandError::Failed {
            command: format!("{args:?}"),
            status: output.status,
            stderr,
        })
    }
}

fn run_jj_command_interactive(
    global_args: &GlobalArgs,
    args: &[&str],
    term: &mut Terminal<impl Backend>,
) -> Result<(), JjCommandError> {
    let mut command = get_jj_command(global_args);
    command.args(args);
    command.stderr(std::process::Stdio::piped());

    terminal::relinquish_terminal().map_err(|e| JjCommandError::Other { err: e })?;
    let mut child = command
        .spawn()
        .map_err(|e| JjCommandError::Other { err: e.into() })?;
    let status = child
        .wait()
        .map_err(|e| JjCommandError::Other { err: e.into() })?;
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .ok_or_else(|| JjCommandError::Other {
            err: anyhow!("No stderr"),
        })?
        .read_to_string(&mut stderr)
        .map_err(|e| JjCommandError::Other { err: e.into() })?;
    terminal::takeover_terminal(term).map_err(|e| JjCommandError::Other { err: e })?;

    if status.success() {
        Ok(())
    } else {
        Err(JjCommandError::Failed {
            command: format!("{args:?}"),
            status,
            stderr,
        })
    }
}

pub fn ensure_valid_repo(repository: &str) -> Result<String> {
    let output = Command::new("jj")
        .env("JJ_CONFIG", "/dev/null")
        .arg("--repository")
        .arg(repository)
        .arg("workspace")
        .arg("root")
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .to_string()
            .trim()
            .to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr = stderr.trim();
        let err_msg = stderr.strip_prefix("Error: ").unwrap_or(stderr);
        bail!("{}", err_msg);
    }
}

fn get_jj_command(global_args: &GlobalArgs) -> Command {
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
        .arg(&global_args.repository);

    if global_args.ignore_immutable {
        command.arg("--ignore-immutable");
    }

    command
}

pub fn log(global_args: &GlobalArgs, revset: &str) -> Result<String, JjCommandError> {
    let args = ["log", "--revisions", revset];
    run_jj_command(global_args, &args)
}

pub fn diff_summary(global_args: &GlobalArgs, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["diff", "--revisions", change_id, "--summary"];
    run_jj_command(global_args, &args)
}

pub fn diff_file(
    global_args: &GlobalArgs,
    change_id: &str,
    file: &str,
) -> Result<String, JjCommandError> {
    let args = ["diff", "--revisions", change_id, file];
    run_jj_command(global_args, &args)
}

pub fn show(
    global_args: &GlobalArgs,
    change_id: &str,
    maybe_file_path: Option<&str>,
    term: &mut Terminal<impl Backend>,
) -> Result<(), JjCommandError> {
    let args = match maybe_file_path {
        None => vec!["show", change_id],
        Some(file_path) => vec!["diff", "--revisions", change_id, file_path],
    };
    run_jj_command_interactive(global_args, &args, term)?;
    Ok(())
}

pub fn describe(
    global_args: &GlobalArgs,
    change_id: &str,
    terminal: &mut Terminal<impl Backend>,
) -> Result<(), JjCommandError> {
    let args = ["describe", change_id];
    run_jj_command_interactive(global_args, &args, terminal)
}

pub fn new(global_args: &GlobalArgs, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["new", change_id];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn abandon(global_args: &GlobalArgs, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["abandon", change_id];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn undo(global_args: &GlobalArgs) -> Result<(), JjCommandError> {
    let args = ["undo"];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn commit(
    global_args: &GlobalArgs,
    term: &mut Terminal<impl Backend>,
) -> Result<(), JjCommandError> {
    let args = ["commit"];
    run_jj_command_interactive(global_args, &args, term)?;
    Ok(())
}

pub fn squash(
    global_args: &GlobalArgs,
    change_id: &str,
    maybe_file_path: Option<&str>,
    term: &mut Terminal<impl Backend>,
) -> Result<(), JjCommandError> {
    let mut args = vec!["squash", "--revision", change_id];
    if let Some(file_path) = maybe_file_path {
        args.push(file_path);
    }
    run_jj_command_interactive(global_args, &args, term)?;
    Ok(())
}

pub fn edit(global_args: &GlobalArgs, change_id: &str) -> Result<(), JjCommandError> {
    let args = ["edit", change_id];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn fetch(global_args: &GlobalArgs) -> Result<(), JjCommandError> {
    let args = ["git", "fetch"];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn push(global_args: &GlobalArgs) -> Result<(), JjCommandError> {
    let args = ["git", "push"];
    run_jj_command(global_args, &args)?;
    Ok(())
}

pub fn bookmark_set_master(
    global_args: &GlobalArgs,
    change_id: &str,
) -> Result<(), JjCommandError> {
    let args = ["bookmark", "set", "master", "--revision", change_id];
    run_jj_command(global_args, &args)?;
    Ok(())
}
