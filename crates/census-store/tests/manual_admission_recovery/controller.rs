use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use rustix::process::{kill_process, Pid, Signal};
use serde::Deserialize;

use super::fixture::{Fixture, CHILD_CASE, CHILD_NAME, CHILD_ROOT};
use super::TestResult;

const INTERVAL: Duration = Duration::from_millis(10);
const MARKER_POLLS: u32 = 3_000;
const REAP_POLLS: u32 = 200;

const PHASE: &str = "journal_context_chunk_committed_before_next";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reached {
    schema: u8,
    pid: u32,
    operation: String,
    pub input_digest: String,
    phase: String,
    index: usize,
}

struct OwnedChild {
    child: Child,
    directory: Option<tempfile::TempDir>,
}

pub(super) fn interrupt(
    directory: tempfile::TempDir,
    fixture: &Fixture,
) -> TestResult<(tempfile::TempDir, Reached)> {
    let marker = directory.path().join("reached.json");
    let child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            CHILD_NAME,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_CASE, CHILD_NAME)
        .env(CHILD_ROOT, &fixture.root)
        .env("CENSUS_NATIVE_WORKER_BOUNDARY", PHASE)
        .env("CENSUS_NATIVE_WORKER_MARKER", &marker)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let mut owned = OwnedChild {
        child,
        directory: Some(directory),
    };
    let reached = wait_marker(&mut owned.child, &marker, &fixture.context_root);
    let stopped = terminate_and_reap(&mut owned.child);
    let (reached, status) = finish(reached, stopped)?;
    check!(eq; status.signal(), Some(Signal::TERM.as_raw()));
    let directory = owned
        .directory
        .take()
        .ok_or("child lost its owned fixture directory")?;
    Ok((directory, reached))
}

fn wait_marker(child: &mut Child, path: &Path, root: &str) -> TestResult<Reached> {
    let started = Instant::now();
    for _ in 0..MARKER_POLLS {
        if let Some(status) = child.try_wait()? {
            return Err(format!("identity fixture exited before reached marker: {status}").into());
        }
        if let Some(reached) = read_marker(path)? {
            check!(eq; reached.schema, 1);
            check!(eq; reached.pid, child.id());
            check!(eq; reached.operation.as_str(), root);
            check!(eq; reached.phase.as_str(), PHASE);
            check!(eq; reached.index, 0);
            check!(
                reached.input_digest.len() == 64
                    && reached
                        .input_digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            );
            return Ok(reached);
        }
        if started.elapsed() >= INTERVAL.saturating_mul(MARKER_POLLS) {
            break;
        }
        std::thread::sleep(INTERVAL);
    }
    Err(format!(
        "identity fixture did not reach {} within bounded marker wait",
        PHASE
    )
    .into())
}

fn read_marker(path: &Path) -> TestResult<Option<Reached>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    check!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 4_096);
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(4_097)
        .read_to_end(&mut bytes)?;
    check!(bytes.len() <= 4_096);
    Ok(Some(serde_json::from_slice(&bytes)?))
}

fn terminate_and_reap(child: &mut Child) -> TestResult<ExitStatus> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }
    let signal = kill_process(Pid::from_child(child), Signal::TERM)
        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() });
    let reaped = wait_reaped(child);
    finish(signal, reaped).map(|(_, status)| status)
}

fn wait_reaped(child: &mut Child) -> TestResult<ExitStatus> {
    let started = Instant::now();
    for _ in 0..REAP_POLLS {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= INTERVAL.saturating_mul(REAP_POLLS) {
            break;
        }
        std::thread::sleep(INTERVAL);
    }
    Err(format!(
        "owned identity child {} was not reaped after SIGTERM",
        child.id()
    )
    .into())
}

fn finish<T, U>(primary: TestResult<T>, cleanup: TestResult<U>) -> TestResult<(T, U)> {
    match (primary, cleanup) {
        (Ok(primary), Ok(cleanup)) => Ok((primary, cleanup)),
        (Err(primary), Ok(_)) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => {
            Err(format!("{primary}; cleanup also failed: {cleanup}").into())
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Err(error) = terminate_and_reap(&mut self.child) {
            let retained = self.directory.take().map(tempfile::TempDir::keep);
            eprintln!("identity child cleanup unverified: {error}; retained fixture: {retained:?}");
        }
    }
}
