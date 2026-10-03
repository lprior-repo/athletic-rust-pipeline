use anyhow::{bail, ensure, Context, Result};
use futures::{stream, StreamExt, TryStreamExt};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::artifacts::{append, now, read_bounded};

mod evidence;
mod output;
use output::OutputLog;
#[cfg(test)]
pub(super) mod tests;

pub struct OwnedProcess {
    child: Child,
    name: String,
    ledger: PathBuf,
    pub log: PathBuf,
    reaped: Option<ExitStatus>,
    started: String,
    requested_signal: Option<i32>,
    term_at: Option<String>,
    output_error: Option<String>,
    ledger_error: Option<String>,
    output: OutputLog,
}

impl OwnedProcess {
    pub fn spawn(
        root: &Path,
        name: &str,
        binary: &Path,
        args: &[String],
        env: &[(&str, &str)],
        failure: Arc<AtomicBool>,
    ) -> Result<Self> {
        let log_path = root.join(format!("{name}.log"));
        let (output, stdout, stderr) = OutputLog::prepare(&log_path, failure)?;
        let mut command = Command::new(binary);
        command
            .args(args)
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .stdout(stdout)
            .stderr(stderr);
        let started = now()?;
        let child = command
            .spawn()
            .with_context(|| format!("starting {name}"))?;
        drop(command);
        let mut owned = Self {
            child,
            name: name.to_string(),
            ledger: root.join("processes.jsonl"),
            log: log_path,
            reaped: None,
            started: started.clone(),
            requested_signal: None,
            term_at: None,
            output_error: None,
            ledger_error: None,
            output,
        };
        if let Err(error) = owned.persist_evidence() {
            let cleanup = owned.stop();
            return Err(error.context(format!("process identity failed; cleanup: {cleanup:?}")));
        }
        if let Err(error) = append(
            &owned.ledger,
            &json!({"event":"spawn", "name":name, "pid":owned.child.id(), "argv":std::iter::once(binary.display().to_string()).chain(args.iter().cloned()).collect::<Vec<_>>(), "env":env, "started":started, "cwd":std::env::current_dir()?}),
        ) {
            owned.retain_ledger_error(&Err(anyhow::anyhow!("{error:#}")));
            let cleanup = owned.stop();
            return Err(error.context(format!("process evidence failed; cleanup: {cleanup:?}")));
        }
        Ok(owned)
    }

    pub fn ensure_running(&mut self) -> Result<()> {
        match self.child.try_wait()? {
            Some(status) => {
                let finalized = self.finish_reap(status);
                bail!(
                    "{} exited early: {status}; log {}; finalization: {finalized:?}",
                    self.name,
                    self.log.display()
                )
            }
            None => self.output.ensure_healthy(),
        }
    }

    pub fn wait(&mut self, ticks: usize) -> Result<ExitStatus> {
        let found = (0..ticks).find_map(|_| match self.child.try_wait() {
            Ok(Some(status)) => Some(Ok(status)),
            Err(error) => Some(Err(error)),
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(100));
                None
            }
        });
        let status =
            found.context("owned child wait exceeded fixed budget; no SIGKILL permitted")??;
        self.finish_reap(status)?;
        Ok(status)
    }

    pub fn stop(&mut self) -> Result<Value> {
        if self.reaped.is_some() {
            self.finish_existing()?;
            let mut evidence = self.evidence();
            evidence
                .as_object_mut()
                .context("already-reaped process evidence is not an object")?
                .insert("already_reaped".into(), json!(true));
            return Ok(evidence);
        }
        if let Some(status) = self.child.try_wait()? {
            self.finish_reap(status)?;
            let mut evidence = self.evidence();
            evidence
                .as_object_mut()
                .context("exited process evidence is not an object")?
                .insert("exit_before_term".into(), json!(status.to_string()));
            return Ok(evidence);
        }
        let sent = now()?;
        self.requested_signal = Some(15);
        self.term_at = Some(sent.clone());
        let retained = self.persist_evidence();
        let signal = send_term(self.child.id(), &self.ledger)?;
        ensure!(
            signal.success(),
            "TERM failed for owned {} pid {}",
            self.name,
            self.child.id()
        );
        let recorded = append(
            &self.ledger,
            &json!({"event":"TERM", "name":self.name, "pid":self.child.id(), "at":sent}),
        );
        self.retain_ledger_error(&recorded);
        let waited = self.wait(650);
        waited?;
        retained?;
        recorded?;
        Ok(self.evidence())
    }

    pub fn output_health(&self) -> Arc<AtomicBool> {
        self.output.health()
    }

    #[cfg(test)]
    pub(super) fn pid(&self) -> u32 {
        self.child.id()
    }

    #[cfg(test)]
    pub(super) fn reaped_status(&self) -> Option<ExitStatus> {
        self.reaped
    }
}

impl Drop for OwnedProcess {
    fn drop(&mut self) {
        if self.reaped.is_some() {
            return;
        }
        match self.stop() {
            Ok(value) => {
                if let Err(error) = append(
                    &self.ledger,
                    &json!({"event":"fallback_cleanup", "result":value}),
                ) {
                    eprintln!("cleanup evidence error: {error:#}");
                }
            }
            Err(error) => eprintln!(
                "TERM/reap cleanup failed for {} pid {}: {error:#}",
                self.name,
                self.child.id()
            ),
        }
    }
}

pub fn command(root: &Path, name: &str, binary: &Path, args: &[String]) -> Result<String> {
    let mut process = OwnedProcess::spawn(
        root,
        name,
        binary,
        args,
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    let status = match process.wait(100) {
        Ok(status) => status,
        Err(error) => {
            let cleanup = process.stop();
            return Err(error.context(format!("bounded command cleanup: {cleanup:?}")));
        }
    };
    let text = read_bounded(&process.log)?;
    ensure!(status.success(), "{name} failed: {status}: {text}");
    Ok(text)
}

#[tracing::instrument(skip_all)]
pub async fn watch_output(node: Arc<AtomicBool>, endpoint: Arc<AtomicBool>) -> Result<()> {
    stream::iter(0..6000)
        .then(|_| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            ensure!(
                !node.load(Ordering::Acquire) && !endpoint.load(Ordering::Acquire),
                "owned child log resource-limit or I/O failure; proceeding to ordered cleanup"
            );
            Ok::<_, anyhow::Error>(())
        })
        .try_for_each(|()| futures::future::ready(Ok(())))
        .await?;
    bail!("output monitor reached fixed 600-second scenario budget")
}

fn send_term(pid: u32, ledger: &Path) -> Result<ExitStatus> {
    let started = now()?;
    let mut child = Command::new("/usr/bin/kill")
        .args(["-TERM", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let helper_pid = child.id();
    let status = (0..100)
        .find_map(|_| match child.try_wait() {
            Ok(Some(status)) => Some(Ok(status)),
            Err(error) => Some(Err(error)),
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(100));
                None
            }
        })
        .with_context(|| {
            format!("TERM helper pid {helper_pid} did not reap within ten seconds")
        })??;
    if let Err(error) = append(
        ledger,
        &json!({"event":"signal_helper_reap", "pid":helper_pid, "argv":["/usr/bin/kill", "-TERM", pid.to_string()], "started":started, "ended":now()?, "status":status.to_string()}),
    ) {
        eprintln!("TERM helper evidence failed after reap; continuing owned child reap: {error:#}");
    }
    Ok(status)
}
