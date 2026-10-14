#[cfg(target_os = "macos")]
use super::{hash_boot_bytes, Digest, StoreError, StoreResult};
use rustix::process::{kill_process, Pid, Signal};
use std::io::{self, Read};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
const SYSCTL: &str = "/usr/sbin/sysctl";
const BOOT_BYTES: usize = 37;
const EXECUTION: WaitBudget = WaitBudget {
    polls: 500,
    interval: Duration::from_millis(10),
};
const CLEANUP: WaitBudget = WaitBudget {
    polls: 100,
    interval: Duration::from_millis(10),
};

#[derive(Clone, Copy)]
struct WaitBudget {
    polls: u32,
    interval: Duration,
}

struct OwnedChild(Child);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Err(error) = terminate_and_reap(&mut self.0) {
            tracing::error!(pid = self.0.id(), %error, "boot identity child cleanup failed");
        }
    }
}

#[cfg(target_os = "macos")]
pub(super) fn read_owner_boot_digest() -> StoreResult<Digest> {
    let mut command = Command::new(SYSCTL);
    command.args(["-n", "kern.bootsessionuuid"]);
    let bytes = run_command(command, EXECUTION).map_err(|source| StoreError::Io {
        path: SYSCTL.into(),
        source,
    })?;
    hash_boot_bytes(&bytes)
}

fn run_command(mut command: Command, budget: WaitBudget) -> io::Result<[u8; BOOT_BYTES]> {
    let (reader, writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let mut child = OwnedChild(
        command
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::from(OwnedFd::from(writer)))
            .stderr(Stdio::null())
            .spawn()?,
    );
    drop(command);
    let result = supervise(&mut child.0, reader, budget);
    finish(result, terminate_and_reap(&mut child.0))
}

fn supervise(
    child: &mut Child,
    mut reader: UnixStream,
    budget: WaitBudget,
) -> io::Result<[u8; BOOT_BYTES]> {
    let started = Instant::now();
    let mut bytes = [0_u8; 38];
    let mut length = 0_usize;
    for _ in 0..budget.polls {
        let ended = read_chunk(&mut reader, &mut bytes, &mut length)?;
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                return Err(io::Error::other(format!(
                    "boot identity command exited with {status}"
                )));
            }
            if ended {
                return parse_boot_session(bytes.get(..length).ok_or_else(invalid_uuid)?);
            }
        }
        if started.elapsed() >= budget.interval.saturating_mul(budget.polls) {
            break;
        }
        std::thread::sleep(budget.interval);
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "boot identity command timed out",
    ))
}

fn read_chunk(
    reader: &mut UnixStream,
    bytes: &mut [u8; 38],
    length: &mut usize,
) -> io::Result<bool> {
    let remaining = bytes.get_mut(*length..).ok_or_else(invalid_uuid)?;
    match reader.read(remaining) {
        Ok(0) => Ok(true),
        Ok(count) => {
            *length = length.checked_add(count).ok_or_else(invalid_uuid)?;
            if *length > BOOT_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "boot identity output exceeds 37 bytes",
                ));
            }
            Ok(false)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

fn parse_boot_session(bytes: &[u8]) -> io::Result<[u8; BOOT_BYTES]> {
    let uuid = bytes.strip_suffix(b"\n").ok_or_else(invalid_uuid)?;
    if uuid.len() != 36
        || !uuid.iter().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
        || uuid.iter().all(|byte| matches!(byte, b'0' | b'-'))
    {
        return Err(invalid_uuid());
    }
    bytes.try_into().map_err(|_| invalid_uuid())
}

fn invalid_uuid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "boot identity is not a nonzero UUID followed by one newline",
    )
}

fn terminate_and_reap(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }
    let signaled = kill_process(Pid::from_child(child), Signal::TERM).map_err(io::Error::from);
    let reaped = wait_reaped(child);
    finish(signaled, reaped)
}

fn wait_reaped(child: &mut Child) -> io::Result<()> {
    let started = Instant::now();
    for _ in 0..CLEANUP.polls {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if started.elapsed() >= CLEANUP.interval.saturating_mul(CLEANUP.polls) {
            break;
        }
        std::thread::sleep(CLEANUP.interval);
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "boot identity child {} was not reaped after SIGTERM",
            child.id()
        ),
    ))
}

fn finish<T>(result: io::Result<T>, cleanup: io::Result<()>) -> io::Result<T> {
    match (result, cleanup) {
        (result, Ok(())) => result,
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => Err(io::Error::new(
            primary.kind(),
            format!("{primary}; child cleanup failed: {cleanup}"),
        )),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
