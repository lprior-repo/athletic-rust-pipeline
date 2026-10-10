use crate::{CrawlError, CrawlResult};
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const CONVERTER_MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
pub const CONVERTER_DEADLINE: Duration = Duration::from_secs(120);

const CONVERTER_POLL: Duration = Duration::from_millis(5);
const CONVERTER_READ_CHUNK: usize = 64 * 1024;

pub fn converter_stdout_capped<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    input: &[u8],
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<Vec<u8>> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| converter_io(command, error))?;
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    let (Some(stdin), Some(stdout)) = (stdin, stdout) else {
        terminate(&mut child, command)?;
        return Err(converter_invariant(command, "did not expose its pipes"));
    };
    let over_cap = AtomicBool::new(false);
    let (status, read, write) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            let mut stdin = stdin;
            stdin.write_all(input)
        });
        let reader = scope.spawn(|| read_capped(stdout, max_output, &over_cap));
        let status = supervise(
            &mut child,
            deadline,
            None,
            Some(&over_cap),
            max_output,
            command,
        );
        (status, reader.join(), writer.join())
    });
    let status = status?;
    let read = read
        .map_err(|_| converter_invariant(command, "reader thread panicked"))?
        .map_err(|error| converter_io(command, error))?;
    let write = write.map_err(|_| converter_invariant(command, "writer thread panicked"))?;
    if !status.success() {
        return Err(converter_exit(command, status));
    }
    match read {
        ReadOutcome::Filled(bytes) => {
            write.map_err(|error| converter_io(command, error))?;
            Ok(bytes)
        }
        ReadOutcome::OverCap => Err(converter_bytes(max_output)),
    }
}

pub fn converter_files_capped<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    watch: &Path,
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| converter_io(command, error))?;
    let status = supervise(
        &mut child,
        deadline,
        Some((watch, max_output)),
        None,
        max_output,
        command,
    )?;
    if !status.success() {
        return Err(converter_exit(command, status));
    }
    Ok(())
}

pub fn read_file_capped(path: &Path, max_bytes: usize) -> CrawlResult<Vec<u8>> {
    let file = std::fs::File::open(path).map_err(|error| converter_io(path, error))?;
    let mut reader = file.take(max_bytes.saturating_add(1) as u64);
    let mut collected = Vec::new();
    reader
        .read_to_end(&mut collected)
        .map_err(|error| converter_io(path, error))?;
    if collected.len() > max_bytes {
        return Err(converter_bytes(max_bytes));
    }
    Ok(collected)
}

enum ReadOutcome {
    Filled(Vec<u8>),
    OverCap,
}

fn read_capped(
    mut stdout: ChildStdout,
    max_bytes: usize,
    over_cap: &AtomicBool,
) -> std::io::Result<ReadOutcome> {
    let mut collected: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; CONVERTER_READ_CHUNK];
    loop {
        let read = stdout.read(&mut chunk)?;
        if read == 0 {
            return Ok(ReadOutcome::Filled(collected));
        }
        let next = collected
            .len()
            .checked_add(read)
            .ok_or_else(|| std::io::Error::other("converter output length overflowed"))?;
        if next > max_bytes {
            over_cap.store(true, Ordering::Relaxed);
            return Ok(ReadOutcome::OverCap);
        }
        collected
            .try_reserve(read)
            .map_err(|_| std::io::Error::other("converter output reservation failed"))?;
        collected.extend_from_slice(&chunk[..read]);
    }
}

fn supervise(
    child: &mut Child,
    deadline: Duration,
    watch: Option<(&Path, usize)>,
    over_cap: Option<&AtomicBool>,
    max_bytes: usize,
    command: &str,
) -> CrawlResult<ExitStatus> {
    let started = Instant::now();
    loop {
        if over_cap.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            terminate(child, command)?;
            return Err(converter_bytes(max_bytes));
        }
        if let Some((path, limit)) = watch {
            let held = std::fs::metadata(path).map_or(0, |meta| meta.len());
            if held > limit as u64 {
                terminate(child, command)?;
                return Err(converter_bytes(limit));
            }
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| converter_io(command, error))?
        {
            return Ok(status);
        }
        if started.elapsed() >= deadline {
            terminate(child, command)?;
            return Err(CrawlError::ConverterDeadline {
                command: command.to_string(),
                deadline,
            });
        }
        std::thread::sleep(CONVERTER_POLL);
    }
}

fn terminate(child: &mut Child, command: &str) -> CrawlResult<ExitStatus> {
    if let Some(status) = child
        .try_wait()
        .map_err(|error| converter_io(command, error))?
    {
        return Ok(status);
    }
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {}
        Err(error) => return Err(converter_io(command, error)),
    }
    child.wait().map_err(|error| converter_io(command, error))
}

fn converter_io(path: impl AsRef<Path>, source: std::io::Error) -> CrawlError {
    CrawlError::Io {
        path: path.as_ref().to_path_buf(),
        source,
    }
}

fn converter_invariant(command: &str, detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("converter {command} {detail}"),
    }
}

fn converter_exit(command: &str, status: ExitStatus) -> CrawlError {
    CrawlError::Io {
        path: PathBuf::from(command),
        source: std::io::Error::other(format!("{command} exited with {status}")),
    }
}

fn converter_bytes(limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "converter output bytes",
        requested: limit.saturating_add(1),
        limit,
    }
}
