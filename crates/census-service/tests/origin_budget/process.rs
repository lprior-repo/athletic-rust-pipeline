use anyhow::{ensure, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use super::{DELAY_MS, PROCESS_DEADLINE};

pub(super) fn binary() -> Result<PathBuf> {
    let binary = match std::env::var_os("BINARY") {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env!("CARGO_BIN_EXE_census-service")),
    };
    Ok(std::fs::canonicalize(binary)?)
}

#[derive(Debug, Serialize)]
pub(super) struct Outcome {
    pub pid: u32,
    pub agent: String,
    pub store: PathBuf,
    pub working_directory: PathBuf,
    pub argv: Vec<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub(super) struct Workflow {
    child: Child,
    pid: u32,
    agent: String,
    store: PathBuf,
    directory: PathBuf,
    argv: Vec<String>,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Workflow {
    pub(super) fn spawn(
        directory: &Path,
        binary: &Path,
        origin: &str,
        name: &str,
        path: &str,
    ) -> Result<Self> {
        let store = directory.join(format!("{name}-store"));
        let agent = format!("CensusOriginBudgetQualification/{name}");
        let stdout = directory.join(format!("{name}.stdout.log"));
        let stderr = directory.join(format!("{name}.stderr.log"));
        let argv = vec![
            "--store".to_string(),
            store
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("non-UTF8 store"))?
                .to_string(),
            "--authorized-host".to_string(),
            "127.0.0.1".to_string(),
            "--delay-ms".to_string(),
            DELAY_MS.to_string(),
            "--user-agent".to_string(),
            agent.clone(),
            "fetch".to_string(),
            "--refresh".to_string(),
            format!("{origin}{path}"),
        ];
        let child = command(directory, binary, &argv, &stdout, &stderr)?.spawn()?;
        let pid = child.id();
        Ok(Self {
            child,
            pid,
            agent,
            store,
            directory: directory.to_path_buf(),
            argv,
            stdout,
            stderr,
        })
    }

    pub(super) fn spawn_replay(directory: &Path, binary: &Path, owner: &Self) -> Result<Self> {
        let mut argv = owner.argv.clone();
        argv.retain(|arg| arg != "--refresh");
        let stdout = directory.join("replay.stdout.log");
        let stderr = directory.join("replay.stderr.log");
        let child = command(directory, binary, &argv, &stdout, &stderr)?.spawn()?;
        let pid = child.id();
        Ok(Self {
            child,
            pid,
            agent: owner.agent.clone(),
            store: owner.store.clone(),
            directory: directory.to_path_buf(),
            argv,
            stdout,
            stderr,
        })
    }

    pub(super) fn pid(&self) -> u32 {
        self.pid
    }
    pub(super) fn store(&self) -> &Path {
        &self.store
    }

    pub(super) fn require_live(&mut self) -> Result<()> {
        ensure!(
            self.child.try_wait()?.is_none(),
            "owner {} exited while target handshake was held",
            self.pid
        );
        Ok(())
    }

    pub(super) async fn finish(&mut self) -> Result<Outcome> {
        let status = tokio::time::timeout(PROCESS_DEADLINE, wait(&mut self.child)).await??;
        Ok(Outcome {
            pid: self.pid,
            agent: self.agent.clone(),
            store: self.store.clone(),
            working_directory: self.directory.clone(),
            argv: self.argv.clone(),
            exit_code: status.code().ok_or_else(|| {
                anyhow::anyhow!("workflow {} exited by signal: {status}", self.pid)
            })?,
            stdout: bounded_log(&self.stdout)?,
            stderr: bounded_log(&self.stderr)?,
        })
    }

    pub(super) async fn cleanup(&mut self) -> Result<()> {
        let observed = self.child.try_wait();
        if matches!(observed, Ok(Some(_))) {
            return Ok(());
        }
        let delivered = self.child.kill();
        let reaped = tokio::time::timeout(PROCESS_DEADLINE, wait(&mut self.child)).await;
        match (observed, delivered, reaped) {
            (Ok(None), Ok(()), Ok(Ok(status))) => {
                ensure!(!status.success(), "killed unfinished child exited successfully");
                Ok(())
            }
            (observed, delivered, reaped) => Err(anyhow::anyhow!(
                "owned child {} cleanup: observation={observed:?}; kill={delivered:?}; reap={reaped:?}", self.pid)),
        }
    }
}

fn command(
    directory: &Path,
    binary: &Path,
    argv: &[String],
    stdout: &Path,
    stderr: &Path,
) -> Result<Command> {
    let mut command = Command::new(binary);
    command
        .args(argv)
        .current_dir(directory)
        .stdin(Stdio::null())
        .env("RUST_LOG", "info")
        .env("NO_COLOR", "1")
        .env("HTTP_PROXY", "http://127.0.0.1:9")
        .env("HTTPS_PROXY", "http://127.0.0.1:9")
        .env("ALL_PROXY", "http://127.0.0.1:9")
        .env("NO_PROXY", "127.0.0.1")
        .env("http_proxy", "http://127.0.0.1:9")
        .env("https_proxy", "http://127.0.0.1:9")
        .env("all_proxy", "http://127.0.0.1:9")
        .env("no_proxy", "127.0.0.1")
        .stdout(Stdio::from(new_log(stdout)?))
        .stderr(Stdio::from(new_log(stderr)?));
    Ok(command)
}

fn new_log(path: &Path) -> Result<std::fs::File> {
    Ok(std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?)
}

fn bounded_log(path: &Path) -> Result<String> {
    ensure!(
        std::fs::metadata(path)?.len() <= 65_536,
        "workflow log exceeded 64KiB: {}",
        path.display()
    );
    let text = std::fs::read_to_string(path)?;
    let ansi = regex::Regex::new(r"\x1b\[[0-9;]*m")?;
    Ok(ansi.replace_all(&text, "").into_owned())
}

async fn wait(child: &mut Child) -> Result<ExitStatus> {
    let mut poll = tokio::time::interval(Duration::from_millis(10));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    for _ in 0..3_000 {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        poll.tick().await;
    }
    Err(anyhow::anyhow!(
        "owned child {} did not exit within bounded completion polls",
        child.id()
    ))
}
