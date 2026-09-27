
use crate::paths;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub struct Cmd {
    program: String,
    args: Vec<String>,
}

impl Cmd {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn render(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(quote)
            .collect::<Vec<String>>()
            .join(" ")
    }

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

    pub fn output(&self) -> Result<String> {
        let rendered = self.render();
        eprintln!("+ {rendered}");
        let output = Command::new(&self.program)
            .args(&self.args)
            .current_dir(paths::repo_root())
            .output()
            .with_context(|| format!("running `{rendered}`"))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            let detail = detail.trim();
            match output.status.code() {
                Some(code) => bail!("`{rendered}` exited with status {code}: {detail}"),
                None => bail!("`{rendered}` was killed by a signal: {detail}"),
            }
        }
        String::from_utf8(output.stdout)
            .with_context(|| format!("`{rendered}` wrote output that is not UTF-8"))
    }
    pub fn capture(self) -> Result<(String, String)> {
        let rendered = self.render();
        eprintln!("+ {rendered}");
        let output = Command::new(&self.program)
            .args(&self.args)
            .current_dir(paths::repo_root())
            .output()
            .with_context(|| format!("running `{rendered}`"))?;
        let stdout = String::from_utf8(output.stdout)
            .with_context(|| format!("`{rendered}` wrote stdout that is not UTF-8"))?;
        let stderr = String::from_utf8(output.stderr)
            .with_context(|| format!("`{rendered}` wrote stderr that is not UTF-8"))?;
        Ok((stdout, stderr))
    }
}

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
