use crate::model::GlobalArgs;
use crate::terminal;
use anyhow::{Result, anyhow};
use ratatui::{Terminal, backend::Backend};
use std::{
    io::Read,
    process::{Command, ExitStatus},
};

#[derive(Debug)]
pub enum JjCommandError {
    Failed {
        _args: String,
        _status: ExitStatus,
        stderr: String,
    },
    Other {
        err: anyhow::Error,
    },
}

impl JjCommandError {
    fn new_failed(args: &[&str], status: ExitStatus, stderr: String) -> Self {
        Self::Failed {
            _args: format!("{args:?}"),
            _status: status,
            stderr: stderr.trim().to_string(),
        }
    }

    fn new_other(err: impl Into<anyhow::Error>) -> Self {
        Self::Other { err: err.into() }
    }
}

impl std::fmt::Display for JjCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed {
                _args,
                _status,
                stderr,
            } => {
                write!(f, "{stderr}")
            }
            Self::Other { err } => err.fmt(f),
        }
    }
}

impl std::error::Error for JjCommandError {}

pub struct JjCommandOutput {
    pub stdout: String,
    pub stderr: String,
}

fn run_jj_command(
    global_args: &GlobalArgs,
    args: &[&str],
) -> Result<JjCommandOutput, JjCommandError> {
    let mut command = get_jj_command(global_args);
    command.args(args);
    let output = command.output().map_err(JjCommandError::new_other)?;

    let stderr = String::from_utf8_lossy(&output.stderr).into();
    if output.status.success() {
        let stdout = String::from_utf8(output.stdout).map_err(JjCommandError::new_other)?;
        Ok(JjCommandOutput { stdout, stderr })
    } else {
        Err(JjCommandError::new_failed(args, output.status, stderr))
    }
}

fn run_jj_command_interactive(
    global_args: &GlobalArgs,
    args: &[&str],
    term: &mut Terminal<impl Backend>,
) -> Result<JjCommandOutput, JjCommandError> {
    let mut command = get_jj_command(global_args);
    command.args(args);
    command.stderr(std::process::Stdio::piped());

    terminal::relinquish_terminal().map_err(JjCommandError::new_other)?;

    let mut child = command.spawn().map_err(JjCommandError::new_other)?;
    let status = child.wait().map_err(JjCommandError::new_other)?;
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .ok_or_else(|| JjCommandError::new_other(anyhow!("No stderr")))?
        .read_to_string(&mut stderr)
        .map_err(JjCommandError::new_other)?;

    terminal::takeover_terminal(term).map_err(JjCommandError::new_other)?;

    if status.success() {
        Ok(JjCommandOutput {
            stdout: "".to_string(),
            stderr,
        })
    } else {
        Err(JjCommandError::new_failed(args, status, stderr))
    }
}

pub fn ensure_valid_repo(repository: &str) -> Result<String, JjCommandError> {
    let args = [
        "--repository",
        repository,
        "workspace",
        "root",
        "--color",
        "always",
    ];
    let output = Command::new("jj")
        .args(args)
        .output()
        .map_err(JjCommandError::new_other)?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout)
            .to_string()
            .trim()
            .to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).into();
        Err(JjCommandError::new_failed(&args, output.status, stderr))
    }
}

fn get_jj_command(global_args: &GlobalArgs) -> Command {
    let mut command = Command::new("jj");
    let args = [
        "--color",
        "always",
        // "--config",
        // "colors.'diff added token'={underline=false}",
        // "--config",
        // "colors.'diff removed token'={underline=false}",
        // "--config",
        // "colors.'diff token'={underline=false}",
        "--config",
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
        "--repository",
        &global_args.repository,
    ];
    command.args(args);

    if global_args.ignore_immutable {
        command.arg("--ignore-immutable");
    }

    command
}

pub fn log(global_args: &GlobalArgs, revset: &str) -> Result<JjCommandOutput, JjCommandError> {
    let args = ["log", "--revisions", revset];
    run_jj_command(global_args, &args)
}

pub fn diff_summary(
    global_args: &GlobalArgs,
    change_id: &str,
) -> Result<JjCommandOutput, JjCommandError> {
    let args = ["diff", "--revisions", change_id, "--summary"];
    run_jj_command(global_args, &args)
}

pub fn diff_file(
    global_args: &GlobalArgs,
    change_id: &str,
    file: &str,
) -> Result<JjCommandOutput, JjCommandError> {
    let args = ["diff", "--revisions", change_id, file];
    run_jj_command(global_args, &args)
}

pub fn show(
    global_args: &GlobalArgs,
    change_id: &str,
    maybe_file_path: Option<&str>,
    term: &mut Terminal<impl Backend>,
) -> Result<String, JjCommandError> {
    let args = match maybe_file_path {
        None => vec!["show", change_id],
        Some(file_path) => vec!["diff", "--revisions", change_id, file_path],
    };
    Ok(run_jj_command_interactive(global_args, &args, term)?.stderr)
}

pub fn describe(
    global_args: &GlobalArgs,
    change_id: &str,
    term: &mut Terminal<impl Backend>,
) -> Result<String, JjCommandError> {
    let args = ["describe", change_id];
    Ok(run_jj_command_interactive(global_args, &args, term)?.stderr)
}

pub fn new(global_args: &GlobalArgs, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["new", change_id];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn new_before(global_args: &GlobalArgs, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["new", "--no-edit", "--insert-before", change_id];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn abandon(global_args: &GlobalArgs, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["abandon", change_id];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn undo(global_args: &GlobalArgs) -> Result<String, JjCommandError> {
    let args = ["undo"];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn commit(
    global_args: &GlobalArgs,
    term: &mut Terminal<impl Backend>,
) -> Result<String, JjCommandError> {
    let args = ["commit"];
    Ok(run_jj_command_interactive(global_args, &args, term)?.stderr)
}

pub fn squash_noninteractive(
    global_args: &GlobalArgs,
    change_id: &str,
    maybe_file_path: Option<&str>,
) -> Result<String, JjCommandError> {
    let mut args = vec!["squash", "--revision", change_id];
    if let Some(file_path) = maybe_file_path {
        args.push(file_path);
    }
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn squash_interactive(
    global_args: &GlobalArgs,
    change_id: &str,
    maybe_file_path: Option<&str>,
    term: &mut Terminal<impl Backend>,
) -> Result<String, JjCommandError> {
    let mut args = vec!["squash", "--revision", change_id];
    if let Some(file_path) = maybe_file_path {
        args.push(file_path);
    }
    Ok(run_jj_command_interactive(global_args, &args, term)?.stderr)
}

pub fn edit(global_args: &GlobalArgs, change_id: &str) -> Result<String, JjCommandError> {
    let args = ["edit", change_id];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn fetch(global_args: &GlobalArgs) -> Result<String, JjCommandError> {
    let args = ["git", "fetch"];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn push(global_args: &GlobalArgs) -> Result<String, JjCommandError> {
    let args = ["git", "push"];
    Ok(run_jj_command(global_args, &args)?.stderr)
}

pub fn bookmark_set_master(
    global_args: &GlobalArgs,
    change_id: &str,
) -> Result<String, JjCommandError> {
    let args = ["bookmark", "set", "master", "--revision", change_id];
    Ok(run_jj_command(global_args, &args)?.stderr)
}
