use super::TestResult;
use std::fmt;
use std::io::{self, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, Command, ExitStatus};
use std::time::Duration;

mod tests;

pub(crate) struct WaitBudget {
    polls: usize,
    interval: Duration,
    read_timeout: Duration,
}

impl WaitBudget {
    pub(crate) const REAL: Self = Self {
        polls: 3000,
        interval: Duration::from_millis(10),
        read_timeout: Duration::from_secs(10),
    };
}

#[derive(Debug)]
struct Cleanup {
    status: io::Result<ExitStatus>,
    inspection_error: Option<io::Error>,
    kill_error: Option<io::Error>,
}

impl Cleanup {
    fn succeeded(&self) -> bool {
        self.status.is_ok() && self.inspection_error.is_none() && self.kill_error.is_none()
    }
}

#[derive(Debug)]
struct ChildFailure {
    primary: Box<dyn std::error::Error>,
    cleanup: Cleanup,
}

impl fmt::Display for ChildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}; child cleanup: {:?}",
            self.primary, self.cleanup
        )
    }
}

impl std::error::Error for ChildFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.primary.as_ref())
    }
}

#[derive(Clone, Copy)]
enum CleanupMode {
    Kill,
    Terminate,
}

struct OwnedChild {
    child: Option<Child>,
    mode: CleanupMode,
    directory: Option<tempfile::TempDir>,
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let cleanup = terminate_and_reap(&mut child, self.mode);
            if !cleanup.succeeded() {
                let retained = self.directory.take().map(tempfile::TempDir::keep);
                eprintln!(
                    "owned fixture child cleanup failed: {cleanup:?}; retained: {retained:?}"
                );
            }
        }
    }
}

pub(super) fn with_child<T>(
    command: Command,
    run: impl FnOnce(&mut Child) -> TestResult<T>,
) -> TestResult<T> {
    with_cleanup(command, CleanupMode::Kill, None, run).map(|(value, _)| value)
}

pub(crate) fn with_terminated_child<T>(
    command: Command,
    directory: tempfile::TempDir,
    run: impl FnOnce(&mut Child) -> TestResult<T>,
) -> TestResult<(T, tempfile::TempDir)> {
    let (value, directory) = with_cleanup(command, CleanupMode::Terminate, Some(directory), run)?;
    Ok((value, directory.ok_or("owned fixture directory missing")?))
}

fn with_cleanup<T>(
    mut command: Command,
    mode: CleanupMode,
    directory: Option<tempfile::TempDir>,
    run: impl FnOnce(&mut Child) -> TestResult<T>,
) -> TestResult<(T, Option<tempfile::TempDir>)> {
    let mut guard = OwnedChild {
        child: Some(command.spawn()?),
        mode,
        directory,
    };
    let result = run(guard
        .child
        .as_mut()
        .ok_or("owned child missing before callback")?);
    let mut child = guard
        .child
        .take()
        .ok_or("owned child missing after callback")?;
    let cleanup = terminate_and_reap(&mut child, mode);
    if !cleanup.succeeded() {
        let retained = guard.directory.take().map(tempfile::TempDir::keep);
        eprintln!("owned fixture cleanup unverified; retained: {retained:?}");
    }
    finish(result, cleanup).map(|value| (value, guard.directory.take()))
}

pub(crate) fn active_intent(
    store: &census_store::Store,
    root: &std::path::Path,
) -> TestResult<census_store::publish::Intent> {
    let namespace = super::super::binding::namespace(root)?;
    store
        .publication_active(&namespace)?
        .ok_or_else(|| "durable publication intent missing".into())
}

fn finish<T>(result: TestResult<T>, cleanup: Cleanup) -> TestResult<T> {
    match result {
        Err(primary) => Err(Box::new(ChildFailure { primary, cleanup })),
        Ok(value) if cleanup.succeeded() => Ok(value),
        Ok(_) => Err(Box::new(ChildFailure {
            primary: "fixture callback succeeded but child cleanup failed".into(),
            cleanup,
        })),
    }
}

fn terminate_and_reap(child: &mut Child, mode: CleanupMode) -> Cleanup {
    let inspection_error = match child.try_wait() {
        Ok(Some(status)) => {
            return Cleanup {
                status: Ok(status),
                inspection_error: None,
                kill_error: None,
            };
        }
        Ok(None) => None,
        Err(error) => Some(error),
    };
    let termination = match mode {
        CleanupMode::Kill => child.kill(),
        CleanupMode::Terminate => rustix::process::kill_process(
            rustix::process::Pid::from_child(child),
            rustix::process::Signal::TERM,
        )
        .map_err(io::Error::from),
    };
    let kill_error = termination.err();
    let status = if kill_error.is_none() {
        match mode {
            CleanupMode::Kill => child.wait(),
            CleanupMode::Terminate => wait_terminated(child),
        }
    } else {
        child.try_wait().and_then(|status| {
            status.ok_or_else(|| {
                io::Error::other("owned child remained running after failed termination")
            })
        })
    };
    Cleanup {
        status,
        inspection_error,
        kill_error,
    }
}

#[derive(Debug)]
enum WaitFailure {
    Timeout,
    Exited(ExitStatus),
}

impl fmt::Display for WaitFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => formatter.write_str("checkpoint accept timed out"),
            Self::Exited(status) => write!(formatter, "child exited before checkpoint: {status}"),
        }
    }
}

impl std::error::Error for WaitFailure {}

pub(crate) fn wait_checkpoint(
    listener: UnixListener,
    child: &mut Child,
    window: &str,
    budget: WaitBudget,
) -> TestResult<UnixStream> {
    listener.set_nonblocking(true)?;
    let mut stream = (0..budget.polls)
        .find_map(|_| accept_or_poll(&listener, child, budget.interval))
        .ok_or(WaitFailure::Timeout)??;
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(budget.read_timeout))?;
    let mut bytes = [0_u8; 7];
    let slice = bytes
        .get_mut(..window.len())
        .ok_or("checkpoint name too long")?;
    stream.read_exact(slice)?;
    if slice != window.as_bytes() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "checkpoint mismatch: expected {:?}, received {slice:?}",
                window.as_bytes()
            ),
        )
        .into());
    }
    Ok(stream)
}

fn accept_or_poll(
    listener: &UnixListener,
    child: &mut Child,
    interval: Duration,
) -> Option<TestResult<UnixStream>> {
    match listener.accept() {
        Ok((stream, _)) => Some(Ok(stream)),
        Err(error) if error.kind() == io::ErrorKind::WouldBlock => poll_child(child, interval),
        Err(error) => Some(Err(error.into())),
    }
}

fn poll_child(child: &mut Child, interval: Duration) -> Option<TestResult<UnixStream>> {
    match child.try_wait() {
        Ok(Some(status)) => Some(Err(WaitFailure::Exited(status).into())),
        Err(error) => Some(Err(error.into())),
        Ok(None) => {
            std::thread::sleep(interval);
            None
        }
    }
}

pub(crate) fn wait_aborted(child: &mut Child) -> TestResult<ExitStatus> {
    Ok(wait_exit(
        child,
        3000,
        "owned child did not exit after abort checkpoint",
    )?)
}

fn wait_terminated(child: &mut Child) -> io::Result<ExitStatus> {
    wait_exit(child, 200, "owned child was not reaped after SIGTERM")
}

fn wait_exit(child: &mut Child, polls: usize, detail: &'static str) -> io::Result<ExitStatus> {
    for _ in 0..polls {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, detail))
}
