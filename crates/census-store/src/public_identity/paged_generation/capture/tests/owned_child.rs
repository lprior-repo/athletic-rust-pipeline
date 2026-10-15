use super::TestResult;
use rustix::process::{kill_process, Pid, Signal};
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

const INTERVAL: Duration = Duration::from_millis(10);
const EXIT_POLLS: u32 = 6_000;
const REAP_POLLS: u32 = 200;

struct OwnedChild {
    child: Child,
    directory: Option<tempfile::TempDir>,
}

pub(super) fn wait(
    child: Child,
    directory: tempfile::TempDir,
) -> TestResult<(tempfile::TempDir, ExitStatus)> {
    let mut owned = OwnedChild {
        child,
        directory: Some(directory),
    };
    let primary = wait_reaped(&mut owned.child, EXIT_POLLS);
    let cleanup = terminate_and_reap(&mut owned.child);
    let (status, _) = finish(primary, cleanup)?;
    let directory = owned
        .directory
        .take()
        .ok_or("capture child lost owned fixture")?;
    Ok((directory, status))
}

fn terminate_and_reap(child: &mut Child) -> TestResult<ExitStatus> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }
    let signal = kill_process(Pid::from_child(child), Signal::TERM)
        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() });
    let reaped = wait_reaped(child, REAP_POLLS);
    finish(signal, reaped).map(|(_, status)| status)
}

fn wait_reaped(child: &mut Child, polls: u32) -> TestResult<ExitStatus> {
    let started = Instant::now();
    for _ in 0..polls {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if started.elapsed() >= INTERVAL.saturating_mul(polls) {
            break;
        }
        std::thread::sleep(INTERVAL);
    }
    Err(format!(
        "capture child {} was not reaped within {} bounded polls",
        child.id(),
        polls
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
            eprintln!("capture child cleanup unverified: {error}; retained fixture: {retained:?}");
        }
    }
}
