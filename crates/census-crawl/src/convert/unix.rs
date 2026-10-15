use super::{converter_bytes, converter_io, ConverterLifecycleError};
use crate::{CrawlError, CrawlResult};
use rustix::process::{waitid, Pid, WaitId, WaitIdOptions};
use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[path = "unix/lifecycle.rs"]
mod lifecycle;
#[path = "unix/pipes.rs"]
mod pipes;

const SUPERVISOR_STEPS: usize = 24_001;
const CONVERTER_POLL: Duration = Duration::from_millis(5);

pub(super) fn stdout<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    input: &[u8],
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<Vec<u8>> {
    let started = Instant::now();
    let mut child = spawn(command, args, Stdio::piped(), Stdio::piped())?;
    let mut pipes = match pipes::Pipes::take(&mut child, input, max_output, command) {
        Ok(pipes) => pipes,
        Err(error) => {
            drop(child.stdin.take());
            drop(child.stdout.take());
            return lifecycle::terminate(&mut child, command).finish(command, Err(error));
        }
    };
    let outcome = supervise(&child, command, started, deadline, || pipes.tick(command));
    pipes.close_stdin();
    let outcome = lifecycle::terminate(&mut child, command).finish(command, outcome);
    pipes.finish(command, outcome)
}

pub(super) fn files<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    watch: &Path,
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<()> {
    let started = Instant::now();
    let mut child = spawn(command, args, Stdio::null(), Stdio::null())?;
    let mut state = WatchState::Awaiting;
    let outcome = supervise(&child, command, started, deadline, || {
        check_watch(watch, max_output, &mut state, WatchPhase::Writing)
    });
    let outcome = lifecycle::terminate(&mut child, command).finish(command, outcome);
    outcome.and_then(|()| check_watch(watch, max_output, &mut state, WatchPhase::Finished))
}

fn spawn<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    stdin: Stdio,
    stdout: Stdio,
) -> CrawlResult<Child> {
    Command::new(command)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|error| converter_io(command, error))
}

fn supervise(
    child: &Child,
    command: &str,
    started: Instant,
    deadline: Duration,
    mut tick: impl FnMut() -> CrawlResult<()>,
) -> CrawlResult<()> {
    for _ in 0..SUPERVISOR_STEPS {
        tick()?;
        if peek(Pid::from_child(child))
            .map_err(|error| converter_io(command, error.into()))?
            .is_some()
        {
            return Ok(());
        }
        if started.elapsed() >= deadline {
            return Err(deadline_error(command, deadline));
        }
        std::thread::sleep(CONVERTER_POLL);
    }
    Err(deadline_error(command, deadline))
}

fn deadline_error(command: &str, deadline: Duration) -> CrawlError {
    CrawlError::ConverterDeadline {
        command: command.to_owned(),
        deadline,
    }
}

fn peek(pid: Pid) -> rustix::io::Result<Option<rustix::process::WaitIdStatus>> {
    waitid(
        WaitId::Pid(pid),
        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
    )
}

enum WatchPhase {
    Writing,
    Finished,
}

enum WatchState {
    Awaiting,
    Present,
}

fn check_watch(
    path: &Path,
    limit: usize,
    state: &mut WatchState,
    phase: WatchPhase,
) -> CrawlResult<()> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return match (phase, &*state) {
                (WatchPhase::Writing, WatchState::Awaiting) => Ok(()),
                _ => Err(converter_io(path, error)),
            };
        }
        Err(error) => return Err(converter_io(path, error)),
    };
    if !metadata.is_file() {
        return Err(converter_io(
            path,
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "converter output is not a regular file",
            ),
        ));
    }
    *state = WatchState::Present;
    let limit_u64 = u64::try_from(limit).map_err(|_| converter_bytes(limit))?;
    if metadata.len() > limit_u64 {
        return Err(converter_bytes(limit));
    }
    Ok(())
}

fn combine<T>(primary: CrawlResult<T>, secondary: CrawlResult<()>) -> CrawlResult<T> {
    match (primary, secondary) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(primary), Err(secondary)) => Err(ConverterLifecycleError::Multiple {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        }
        .into()),
    }
}
