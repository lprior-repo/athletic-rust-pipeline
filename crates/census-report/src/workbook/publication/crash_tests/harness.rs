use super::TestResult;
use std::fmt;
use std::io::{self, Read};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Child, Command, ExitStatus};
use std::time::Duration;

mod tests;

pub(super) struct WaitBudget {
    polls: usize,
    interval: Duration,
    read_timeout: Duration,
}

impl WaitBudget {
    pub(super) const REAL: Self = Self {
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

struct OwnedChild(Option<Child>);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let cleanup = terminate_and_reap(&mut child);
            if !cleanup.succeeded() {
                eprintln!("owned fixture child cleanup failed: {cleanup:?}");
            }
        }
    }
}

pub(super) fn with_child<T>(
    mut command: Command,
    run: impl FnOnce(&mut Child) -> TestResult<T>,
) -> TestResult<T> {
    let mut guard = OwnedChild(Some(command.spawn()?));
    let result = run(guard
        .0
        .as_mut()
        .ok_or("owned child missing before callback")?);
    let mut child = guard.0.take().ok_or("owned child missing after callback")?;
    let cleanup = terminate_and_reap(&mut child);
    finish(result, cleanup)
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

fn terminate_and_reap(child: &mut Child) -> Cleanup {
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
    let kill_error = child.kill().err();
    let status = if kill_error.is_none() {
        child.wait()
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

pub(super) fn wait_checkpoint(
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
