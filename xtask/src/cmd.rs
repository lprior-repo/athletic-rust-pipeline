//! One child process, rendered exactly as it is executed.
//!
//! Subcommands never reimplement a tool: they build a [`Cmd`], which prints the command line a shell
//! would need and then runs that same argument vector from the repository root.

use crate::paths;
use anyhow::{bail, Context, Result};
use std::process::Command;

/// A child process: a program plus its argument vector.
pub struct Cmd {
    program: String,
    args: Vec<String>,
}

impl Cmd {
    /// Start a command for `program`.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    /// Append one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append several arguments.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// The command line as a shell would need it: what [`Cmd::run`] prints and then executes.
    pub fn render(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(quote)
            .collect::<Vec<String>>()
            .join(" ")
    }

    /// Print the command, then run it from the repository root.
    ///
    /// A non-zero exit is an error naming the child's exit code; a child killed by a signal is an
    /// error naming the signal, because there is no code to report.
    pub fn run(&self) -> Result<()> {
        let rendered = self.render();
        println!("+ {rendered}");
        let status = Command::new(&self.program)
            .args(&self.args)
            .current_dir(paths::repo_root())
            .status()
            .with_context(|| format!("running `{rendered}`"))?;
        if status.success() {
            return Ok(());
        }
        match status.code() {
            Some(code) => bail!("`{rendered}` exited with status {code}"),
            None => bail!("`{rendered}` was killed by a signal"),
        }
    }
}

/// Quote one argument the way a POSIX shell needs it; arguments that are already plain stay bare.
fn quote(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '-' | '_' | '.' | '/' | '=' | ',' | ':' | '@' | '+')
        });
    if plain {
        return arg.to_string();
    }
    let mut quoted = String::with_capacity(arg.len().saturating_add(2));
    quoted.push('\'');
    for c in arg.chars() {
        if c == '\'' {
            quoted.push_str("'\\''");
        } else {
            quoted.push(c);
        }
    }
    quoted.push('\'');
    quoted
}
