use super::super::{artifacts, cancellation::Cancellation};
use anyhow::{ensure, Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub(super) const MARKER: &[u8] = b"\nRESOURCE_LIMIT: child output exceeds 32 MiB; qualification failed; remaining output discarded during bounded cleanup\n";

struct Sink {
    socket: UnixStream,
    file: File,
    size: u64,
    eof: bool,
    exceeded: bool,
}

impl Sink {
    fn prepare(path: &Path) -> Result<(Self, Stdio)> {
        let file = OpenOptions::new().create_new(true).write(true).open(path)?;
        let (socket, writer) = UnixStream::pair()?;
        socket.set_nonblocking(true)?;
        let writer: OwnedFd = writer.into();
        Ok((
            Self {
                socket,
                file,
                size: 0,
                eof: false,
                exceeded: false,
            },
            Stdio::from(writer),
        ))
    }

    fn drain(&mut self) -> Result<()> {
        let mut buffer = [0_u8; 8192];
        (0..64)
            .find_map(|_| match self.socket.read(&mut buffer) {
                Ok(0) => {
                    self.eof = true;
                    Some(Ok(()))
                }
                Ok(count) => match buffer
                    .get(..count)
                    .context("capture read exceeds buffer")
                    .and_then(|bytes| self.write(bytes))
                {
                    Ok(()) => None,
                    Err(error) => Some(Err(error)),
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Some(Ok(())),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => None,
                Err(error) => Some(Err(error.into())),
            })
            .map_or(Ok(()), |result| result)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        if self.exceeded {
            return Ok(());
        }
        let next = self
            .size
            .checked_add(u64::try_from(bytes.len())?)
            .context("capture length overflow")?;
        let budget = artifacts::LIMIT
            .checked_sub(u64::try_from(MARKER.len())?)
            .context("capture marker exceeds budget")?;
        if next > budget {
            self.file.write_all(MARKER)?;
            self.file.sync_data()?;
            self.exceeded = true;
            return Ok(());
        }
        self.file.write_all(bytes)?;
        self.size = next;
        Ok(())
    }
}

pub struct Capture {
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl Capture {
    pub fn start(
        command: &mut Command,
        log: &Path,
        errors: &Path,
        token: Cancellation,
    ) -> Result<Self> {
        let (stdout, output) = Sink::prepare(log)?;
        let (stderr, error) = Sink::prepare(errors)?;
        command.stdin(Stdio::null()).stdout(output).stderr(error);
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let failure = Arc::clone(&failed);
        let evidence = errors.with_extension("resource.json");
        let worker = std::thread::Builder::new().name("qualification-capture".into()).spawn(move || {
            let result = pump(stdout, stderr, &stopping, &failure, &token);
            if let Err(error) = &result {
                failure.store(true, Ordering::Release);
                token.fail_resource();
                artifacts::publish(&evidence, &serde_json::json!({"failure":format!("{error:#}"),"limit_bytes":artifacts::LIMIT,"at":artifacts::now()}))?;
            }
            result
        })?;
        Ok(Self {
            stop,
            failed,
            worker: Some(worker),
        })
    }

    pub fn check(&self) -> Result<()> {
        ensure!(
            !self.failed.load(Ordering::Acquire),
            "child capture resource limit or I/O failure"
        );
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker
                .join()
                .map_err(|_| anyhow::anyhow!("capture worker panicked"))??;
        }
        self.check()
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        if self.worker.is_some() {
            if let Err(error) = self.finish() {
                eprintln!("capture cleanup failed: {error:#}");
            }
        }
    }
}

fn pump(
    mut output: Sink,
    mut errors: Sink,
    stop: &AtomicBool,
    failed: &AtomicBool,
    token: &Cancellation,
) -> Result<()> {
    let mut finalizing = None;
    (0..400_001)
        .find_map(|_| {
            let drained = output.drain().and_then(|()| errors.drain());
            if let Err(error) = drained {
                return Some(Err(error));
            }
            if output.exceeded || errors.exceeded {
                failed.store(true, Ordering::Release);
                token.fail_resource();
            }
            if output.eof && errors.eof {
                return Some(Ok(()));
            }
            if stop.load(Ordering::Acquire) && finalizing.is_none() {
                finalizing = Some(Instant::now());
            }
            if finalizing.is_some_and(|start| start.elapsed() >= Duration::from_secs(2)) {
                return Some(Err(anyhow::anyhow!(
                    "child output descendants retained descriptors after bounded drain"
                )));
            }
            std::thread::sleep(Duration::from_millis(10));
            None
        })
        .context("capture exceeded fixed worker budget")??;
    output.file.sync_all()?;
    errors.file.sync_all()?;
    ensure!(
        !output.exceeded && !errors.exceeded,
        "RESOURCE_LIMIT: child log exceeded bounded evidence budget"
    );
    Ok(())
}
