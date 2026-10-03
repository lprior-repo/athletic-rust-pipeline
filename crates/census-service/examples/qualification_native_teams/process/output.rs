use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::super::artifacts::MAX_ARTIFACT;

pub(super) struct OutputLog {
    stopping: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Value>>>,
    result: Option<Value>,
}

impl OutputLog {
    pub(super) fn prepare(path: &Path, failed: Arc<AtomicBool>) -> Result<(Self, Stdio, Stdio)> {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?;
        let (reader, writer) = UnixStream::pair()?;
        reader.set_read_timeout(Some(Duration::from_millis(100)))?;
        let stderr = Stdio::from(OwnedFd::from(writer.try_clone()?));
        let stdout = Stdio::from(OwnedFd::from(writer));
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopping);
        let failure = Arc::clone(&failed);
        let worker = std::thread::Builder::new()
            .name("qualification-log-drain".into())
            .spawn(move || {
                let result = drain(reader, file, &stop, &failure);
                if result.is_err() {
                    failure.store(true, Ordering::Release);
                }
                result
            })?;
        Ok((
            Self {
                stopping,
                failed,
                worker: Some(worker),
                result: None,
            },
            stdout,
            stderr,
        ))
    }
    pub(super) fn health(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.failed)
    }

    pub(super) fn evidence(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    pub(super) fn ensure_healthy(&self) -> Result<()> {
        ensure!(
            !self.failed.load(Ordering::Acquire),
            "child log resource-limit or I/O failure; capped evidence retained"
        );
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<Value> {
        self.stopping.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker
                .join()
                .map_err(|_| anyhow::anyhow!("owned child log drain panicked"))?;
            self.result = Some(match result {
                Ok(value) => value,
                Err(error) => json!({"drained":false,"error":format!("{error:#}")}),
            });
        }
        let result = self
            .result
            .as_ref()
            .context("child output drain evidence absent")?;
        ensure!(
            result.get("drained").and_then(Value::as_bool) == Some(true),
            "child log drain failed: {result}"
        );
        self.ensure_healthy()?;
        Ok(result.clone())
    }
}

impl Drop for OutputLog {
    fn drop(&mut self) {
        if self.worker.is_some() {
            if let Err(error) = self.finish() {
                eprintln!("owned log drain fallback failed: {error:#}");
            }
        }
    }
}

fn drain(
    mut reader: UnixStream,
    mut file: std::fs::File,
    stop: &AtomicBool,
    failed: &AtomicBool,
) -> Result<Value> {
    let started = Instant::now();
    let mut written = 0_u64;
    let mut exceeded = false;
    let mut buffer = [0_u8; 8192];
    let mut stopping_reads = 0_usize;
    let terminal = (0..1_000_000)
        .find_map(|_| {
            if let Err(error) = stopping_budget(stop, &mut stopping_reads) {
                failed.store(true, Ordering::Release);
                return Some(Err(error));
            }
            if started.elapsed() >= Duration::from_secs(900) {
                failed.store(true, Ordering::Release);
                return Some(Err(anyhow::anyhow!(
                    "child output drain exceeded 900-second ownership budget"
                )));
            }
            match reader.read(&mut buffer) {
                Ok(0) => Some(Ok(true)),
                Ok(length) => {
                    let result =
                        retain_chunk(&mut file, &buffer, length, &mut written, &mut exceeded);
                    if exceeded || result.is_err() {
                        failed.store(true, Ordering::Release);
                    }
                    result.err().map(Err)
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    stop.load(Ordering::Acquire).then_some(Ok(false))
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => None,
                Err(error) => {
                    failed.store(true, Ordering::Release);
                    Some(Err(error.into()))
                }
            }
        })
        .context("child log drain exhausted fixed read budget")??;
    file.sync_all()?;
    ensure!(
        terminal,
        "child log drain stopped without EOF; inherited output owner remains open"
    );
    ensure!(!exceeded, "child log resource-limit: stdout/stderr exceeded 32 MiB; capped prefix retained and remaining output drained");
    Ok(
        json!({"drained":true,"eof":true,"written_bytes":written,"limit_bytes":MAX_ARTIFACT,"resource_limit_exceeded":false}),
    )
}

fn stopping_budget(stop: &AtomicBool, reads: &mut usize) -> Result<()> {
    if stop.load(Ordering::Acquire) {
        *reads = reads.saturating_add(1);
        ensure!(
            *reads <= 4096,
            "child output final drain exceeded 4096-read ownership budget"
        );
    }
    Ok(())
}

fn retain_chunk(
    file: &mut std::fs::File,
    buffer: &[u8],
    length: usize,
    written: &mut u64,
    exceeded: &mut bool,
) -> Result<()> {
    let bytes = buffer
        .get(..length)
        .context("child output read exceeded fixed buffer")?;
    let final_size = written
        .checked_add(u64::try_from(length)?)
        .context("child output size overflow")?;
    if final_size > MAX_ARTIFACT || *exceeded {
        *exceeded = true;
        return Ok(());
    }
    file.write_all(bytes)?;
    *written = final_size;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom};

    #[test]
    fn child_output_bound_retains_prefix_and_rejects_overflow() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("child.log");
        let mut file = std::fs::File::create(&path)?;
        let mut written = MAX_ARTIFACT.checked_sub(4).context("limit underflow")?;
        file.set_len(written)?;
        file.seek(SeekFrom::End(0))?;
        let mut exceeded = false;
        retain_chunk(&mut file, b"last", 4, &mut written, &mut exceeded)?;
        assert_eq!(written, MAX_ARTIFACT);
        assert!(!exceeded);
        retain_chunk(&mut file, b"overflow", 8, &mut written, &mut exceeded)?;
        assert!(exceeded);
        assert_eq!(written, MAX_ARTIFACT);
        assert_eq!(file.metadata()?.len(), MAX_ARTIFACT);
        Ok(())
    }

    #[test]
    fn final_output_drain_has_a_separate_fixed_read_budget() -> Result<()> {
        let stop = AtomicBool::new(false);
        let mut reads = 0;
        (0..5000).try_for_each(|_| stopping_budget(&stop, &mut reads))?;
        assert_eq!(reads, 0);
        stop.store(true, Ordering::Release);
        (0..4096).try_for_each(|_| stopping_budget(&stop, &mut reads))?;
        assert_eq!(reads, 4096);
        let error = stopping_budget(&stop, &mut reads)
            .err()
            .context("unbounded final output drain accepted")?;
        assert_eq!(reads, 4097);
        assert_eq!(
            error.to_string(),
            "child output final drain exceeded 4096-read ownership budget"
        );
        Ok(())
    }
}
