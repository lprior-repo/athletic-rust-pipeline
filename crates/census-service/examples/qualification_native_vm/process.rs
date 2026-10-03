use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use super::{artifacts, cancellation};

mod capture;
mod deadline;
mod stop;
#[cfg(test)]
mod tests;

pub(super) use deadline::{command_until, DeadlineExpired};

#[derive(Clone, Copy)]
enum Role {
    Owned,
    SignalDelivery,
}

pub struct Process {
    child: Child,
    root: PathBuf,
    label: String,
    generation: String,
    status: Option<ExitStatus>,
    term_requested: bool,
    role: Role,
    capture: capture::Capture,
    pub log: PathBuf,
}

pub fn generation() -> Result<String> {
    let value = String::from_utf8(artifacts::read(Path::new("/proc/sys/kernel/random/uuid"))?)?;
    let value = value.trim();
    ensure!(
        value.len() == 36
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'),
        "invalid process incarnation generation"
    );
    Ok(value.into())
}

impl Process {
    pub fn spawn(root: &Path, label: &str, command: &mut Command) -> Result<Self> {
        let mut owned = Self::launch(root, label, command, Role::Owned)?;
        if let Err(error) = artifacts::append(
            &root.join("processes.jsonl"),
            &json!({"event":"spawn","identity":owned.identity(),"command":format!("{command:?}"),"at":artifacts::now()}),
        ) {
            let cleanup = owned.stop();
            artifacts::publish(
                &root.join(format!("process-{}.json", owned.generation)),
                &json!({"failure":format!("{error:#}"),"cleanup":format!("{cleanup:?}"),"evidence":owned.evidence()}),
            )?;
            return Err(error);
        }
        Ok(owned)
    }

    fn launch(root: &Path, label: &str, command: &mut Command, role: Role) -> Result<Self> {
        cancellation::checkpoint()?;
        ensure!(
            !label.is_empty()
                && label.len() <= 128
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_')),
            "invalid owned process label"
        );
        let generation = generation()?;
        let log = root.join(format!("{label}-{generation}.log"));
        let errors = root.join(format!("{label}-{generation}.stderr.log"));
        let capture = capture::Capture::start(command, &log, &errors, cancellation::current())?;
        let spawned = command
            .spawn()
            .with_context(|| format!("spawning {label}: {command:?}"));
        command.stdout(Stdio::null()).stderr(Stdio::null());
        let child = spawned?;
        Ok(Self {
            child,
            root: root.into(),
            label: label.into(),
            generation,
            status: None,
            term_requested: false,
            role,
            capture,
            log,
        })
    }

    pub fn identity(&self) -> Value {
        json!({"pid":self.child.id(),"label":self.label,"generation":self.generation,"log":self.log})
    }

    pub fn evidence(&self) -> Value {
        json!({"identity":self.identity(),"reaped":self.status.is_some(),"exit":self.status.map(|status| json!({"description":status.to_string(),"code":status.code(),"signal":status.signal(),"core_dumped":status.core_dumped(),"success":status.success()})),"requested_signal":self.term_requested.then_some(15)})
    }

    fn observe(&mut self) -> Result<Option<ExitStatus>> {
        if let Some(status) = self.status {
            return Ok(Some(status));
        }
        let Some(status) = self.child.try_wait()? else {
            return Ok(None);
        };
        self.status = Some(status);
        self.finish_reap()?;
        Ok(Some(status))
    }

    fn finish_reap(&mut self) -> Result<()> {
        let drained = self.capture.finish();
        let recorded = artifacts::append(
            &self.root.join("processes.jsonl"),
            &json!({"event":"reap","evidence":self.evidence(),"capture":format!("{drained:?}"),"at":artifacts::now()}),
        );
        recorded?;
        drained
    }

    pub fn wait(&mut self, ticks: usize) -> Result<ExitStatus> {
        ensure!(ticks <= 36_000, "child wait exceeds fixed budget");
        (0..ticks)
            .find_map(|_| {
                let checked = cancellation::checkpoint().and_then(|()| self.capture.check());
                if let Err(error) = checked {
                    return Some(Err(error));
                }
                match self.observe() {
                    Ok(Some(status)) => Some(Ok(status)),
                    Err(error) => Some(Err(error)),
                    Ok(None) => {
                        std::thread::sleep(Duration::from_millis(100));
                        None
                    }
                }
            })
            .context("owned child deadline expired; no SIGKILL issued")?
    }

    #[cfg(test)]
    pub fn running(&mut self) -> Result<()> {
        cancellation::checkpoint()?;
        if let Some(status) = self.health_status()? {
            bail!("{} exited unexpectedly: {status}", self.label);
        }
        Ok(())
    }

    pub(super) fn health_status(&mut self) -> Result<Option<ExitStatus>> {
        self.capture.check()?;
        self.observe()
    }
}

pub fn command(root: &Path, label: &str, command: &mut Command, ticks: usize) -> Result<String> {
    let mut process = Process::spawn(root, label, command)?;
    let waited = process.wait(ticks);
    if let Err(error) = waited {
        let stopped = process.stop();
        bail!(
            "{label} failed while waiting: {error:#}; cleanup={stopped:?}; evidence={}",
            process.evidence()
        );
    }
    let status = waited?;
    let output = String::from_utf8(artifacts::read(&process.log)?)?;
    ensure!(status.success(), "{label} failed: {status}: {output}");
    Ok(output)
}
