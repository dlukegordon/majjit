use crate::model::GlobalArgs;
use crate::terminal;
use anyhow::{Result, anyhow};
use ratatui::{Terminal, prelude::CrosstermBackend};
use std::{
    io::{Read, Stdout},
    process::Command,
};

pub struct JjCommand<'a> {
    args: Vec<String>,
    global_args: GlobalArgs,
    interactive_term: Option<&'a mut Terminal<CrosstermBackend<Stdout>>>,
    return_output: ReturnOutput,
}

impl std::fmt::Display for JjCommand<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "> jj {:?}", self.args)
    }
}

impl<'a> JjCommand<'a> {
    fn _new(
        args: &[&str],
        global_args: GlobalArgs,
        interactive_term: Option<&'a mut Terminal<CrosstermBackend<Stdout>>>,
        return_output: ReturnOutput,
    ) -> Self {
        Self {
            args: args.iter().map(|a| a.to_string()).collect(),
            global_args,
            interactive_term,
            return_output,
        }
    }

    pub fn run(&mut self) -> Result<String, JjCommandError> {
        let output = match &self.interactive_term {
            None => self.run_noninteractive(),
            Some(..) => self.run_interactive(),
        }?;
        match self.return_output {
            ReturnOutput::Stdout => Ok(output.stdout),
            ReturnOutput::Stderr => Ok(output.stderr),
        }
    }

    fn run_noninteractive(&self) -> Result<JjCommandOutput, JjCommandError> {
        let mut command = self.base_command();
        command.args(self.args.clone());
        let output = command.output().map_err(JjCommandError::new_other)?;

        let stderr = String::from_utf8_lossy(&output.stderr).into();
        if output.status.success() {
            let stdout = String::from_utf8(output.stdout).map_err(JjCommandError::new_other)?;
            Ok(JjCommandOutput { stdout, stderr })
        } else {
            Err(JjCommandError::new_failed(stderr))
        }
    }

    fn run_interactive(&mut self) -> Result<JjCommandOutput, JjCommandError> {
        let mut command = self.base_command();
        command.args(self.args.clone());
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

        let term = self.interactive_term.as_mut().unwrap();
        terminal::takeover_terminal(term).map_err(JjCommandError::new_other)?;

        if status.success() {
            Ok(JjCommandOutput {
                stdout: "".to_string(),
                stderr,
            })
        } else {
            Err(JjCommandError::new_failed(stderr))
        }
    }

    fn base_command(&self) -> Command {
        let mut command = Command::new("jj");
        let args = [
            "--color",
            "always",
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
            &self.global_args.repository,
        ];
        command.args(args);

        if self.global_args.ignore_immutable {
            command.arg("--ignore-immutable");
        }

        command
    }

    pub fn log(revset: &str, global_args: GlobalArgs) -> Self {
        let args = ["log", "--revisions", revset];
        Self::_new(&args, global_args, None, ReturnOutput::Stdout)
    }

    pub fn diff_summary(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["diff", "--revisions", change_id, "--summary"];
        Self::_new(&args, global_args, None, ReturnOutput::Stdout)
    }

    pub fn diff_file(change_id: &str, file: &str, global_args: GlobalArgs) -> Self {
        let args = ["diff", "--revisions", change_id, file];
        Self::_new(&args, global_args, None, ReturnOutput::Stdout)
    }

    pub fn show(
        change_id: &str,
        maybe_file_path: Option<&str>,
        global_args: GlobalArgs,
        term: &'a mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Self {
        let args = match maybe_file_path {
            None => vec!["show", change_id],
            Some(file_path) => vec!["diff", "--revisions", change_id, file_path],
        };
        Self::_new(&args, global_args, Some(term), ReturnOutput::Stderr)
    }

    pub fn describe(
        change_id: &str,
        global_args: GlobalArgs,
        term: &'a mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Self {
        let args = ["describe", change_id];
        Self::_new(&args, global_args, Some(term), ReturnOutput::Stderr)
    }

    pub fn new(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["new", change_id];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn new_before(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["new", "--no-edit", "--insert-before", change_id];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn abandon(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["abandon", change_id];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn undo(global_args: GlobalArgs) -> Self {
        let args = ["undo"];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn commit(
        global_args: GlobalArgs,
        term: &'a mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Self {
        let args = ["commit"];
        Self::_new(&args, global_args, Some(term), ReturnOutput::Stderr)
    }

    pub fn squash_noninteractive(
        change_id: &str,
        maybe_file_path: Option<&str>,
        global_args: GlobalArgs,
    ) -> Self {
        let mut args = vec!["squash", "--revision", change_id];
        if let Some(file_path) = maybe_file_path {
            args.push(file_path);
        }
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn squash_interactive(
        change_id: &str,
        maybe_file_path: Option<&str>,
        global_args: GlobalArgs,
        term: &'a mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Self {
        let mut args = vec!["squash", "--revision", change_id];
        if let Some(file_path) = maybe_file_path {
            args.push(file_path);
        }
        Self::_new(&args, global_args, Some(term), ReturnOutput::Stderr)
    }

    pub fn edit(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["edit", change_id];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn fetch(global_args: GlobalArgs) -> Self {
        let args = ["git", "fetch"];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn push(global_args: GlobalArgs) -> Self {
        let args = ["git", "push"];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
    }

    pub fn bookmark_set_master(change_id: &str, global_args: GlobalArgs) -> Self {
        let args = ["bookmark", "set", "master", "--revision", change_id];
        Self::_new(&args, global_args, None, ReturnOutput::Stderr)
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
            Err(JjCommandError::new_failed(stderr))
        }
    }
}

#[derive(Debug)]
enum ReturnOutput {
    Stdout,
    Stderr,
}

#[derive(Debug)]
pub enum JjCommandError {
    Failed { stderr: String },
    Other { err: anyhow::Error },
}

impl JjCommandError {
    fn new_failed(stderr: String) -> Self {
        Self::Failed {
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
            Self::Failed { stderr } => {
                write!(f, "{stderr}")
            }
            Self::Other { err } => err.fmt(f),
        }
    }
}

impl std::error::Error for JjCommandError {}

#[derive(Debug)]
pub struct JjCommandOutput {
    pub stdout: String,
    pub stderr: String,
}
