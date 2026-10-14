use std::fs::File;
use std::io::Write;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use census_domain::{model::SourceMeetRef, UsJurisdiction};
use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};

use super::{owner, wire, Partition};
use crate::restate_services::results_arms::budget::{self, ScratchPermit, SpillAdmission};

pub(super) struct Writer {
    file: File,
    path: PathBuf,
    jurisdiction: UsJurisdiction,
    buffer: Vec<u8>,
    _scratch: ScratchPermit,
    admission: Arc<SpillAdmission>,
    bytes: u64,
    record_bytes: usize,
    rows: u64,
    digest: Sha256,
    failure: Option<StoreError>,
}

impl Writer {
    pub(super) fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }

    pub(super) fn new(
        directory: &mut owner::Directory,
        jurisdiction: UsJurisdiction,
        admission: Arc<SpillAdmission>,
    ) -> StoreResult<Self> {
        let mut scratch = ScratchPermit::new(Arc::clone(&admission));
        let buffer = scratch.buffer(budget::SPILL_BUFFER_BYTES)?;
        let (path, file) = directory.create_file(jurisdiction.code())?;
        Ok(Self {
            file,
            path,
            jurisdiction,
            buffer,
            _scratch: scratch,
            admission,
            bytes: 0,
            record_bytes: 0,
            rows: 0,
            digest: Sha256::new(),
            failure: None,
        })
    }

    pub(super) fn push(&mut self, row: &SourceMeetRef) -> StoreResult<()> {
        if row.jurisdiction != self.jurisdiction {
            return Err(owner::refused("results spill writer jurisdiction mismatch"));
        }
        self.record_bytes = 0;
        let next = self
            .rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        let framed = wire::Borrowed::new(row, next)?;
        if let Err(source) = serde_json::to_writer(&mut *self, &framed) {
            return Err(match self.failure.take() {
                Some(error) => error,
                None => StoreError::Json {
                    detail: "results spill row serialization failed".to_string(),
                    source,
                },
            });
        }
        self.append(b"\n")?;
        self.rows = next;
        Ok(())
    }

    pub(super) fn finish(mut self) -> StoreResult<Partition> {
        self.flush_buffer()?;
        let metadata = self
            .file
            .metadata()
            .map_err(|source| owner::io_error(&self.path, source))?;
        let identity = owner::identity(&metadata)?;
        if metadata.len() != self.bytes {
            return Err(owner::refused("results spill writer length mismatch"));
        }
        drop(self.file);
        let file = owner::reopen(&self.path, identity, self.bytes)?;
        Ok(Partition {
            file,
            path: self.path,
            jurisdiction: self.jurisdiction,
            bytes: self.bytes,
            rows: self.rows,
            digest: self.digest.finalize().into(),
        })
    }

    fn append(&mut self, bytes: &[u8]) -> StoreResult<()> {
        let next = self
            .record_bytes
            .checked_add(bytes.len())
            .ok_or(StoreError::CounterOverflow)?;
        if next > budget::SPILL_RECORD_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: next,
                max: budget::SPILL_RECORD_BYTES,
            });
        }
        self.record_bytes = next;
        bytes
            .chunks(budget::SPILL_BUFFER_BYTES)
            .try_for_each(|chunk| self.append_chunk(chunk))
    }

    fn append_chunk(&mut self, chunk: &[u8]) -> StoreResult<()> {
        let available = self
            .buffer
            .capacity()
            .checked_sub(self.buffer.len())
            .ok_or(StoreError::CounterOverflow)?;
        if chunk.len() > available {
            self.flush_buffer()?;
        }
        self.buffer.extend_from_slice(chunk);
        Ok(())
    }

    fn flush_buffer(&mut self) -> StoreResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let next = self
            .bytes
            .checked_add(u64::try_from(self.buffer.len()).map_err(|_| StoreError::CounterOverflow)?)
            .ok_or(StoreError::CounterOverflow)?;
        write_buffer(&mut self.file, &self.path, &self.buffer, &self.admission)?;
        self.digest.update(&self.buffer);
        self.bytes = next;
        self.buffer.clear();
        Ok(())
    }
}

fn write_buffer(
    file: &mut impl Write,
    path: &Path,
    buffer: &[u8],
    admission: &SpillAdmission,
) -> StoreResult<()> {
    admission.charge(u64::try_from(buffer.len()).map_err(|_| StoreError::CounterOverflow)?)?;
    let mut offset = 0_usize;
    let progress = (0..budget::SPILL_WRITE_CALLS).try_fold((), |(), _| {
        if offset == buffer.len() {
            return ControlFlow::Break(Ok(()));
        }
        let result = admission.before_write().and_then(|()| {
            let remaining = buffer.get(offset..).ok_or_else(budget::allocation)?;
            file.write(remaining)
                .map_err(|source| owner::io_error(path, source))
        });
        match result {
            Ok(0) => {
                ControlFlow::Break(Err(owner::refused("results spill write made no progress")))
            }
            Ok(count) => match admission.after_write(count).and_then(|()| {
                offset = offset
                    .checked_add(count)
                    .ok_or(StoreError::CounterOverflow)?;
                Ok(())
            }) {
                Ok(()) if offset == buffer.len() => ControlFlow::Break(Ok(())),
                Ok(()) => ControlFlow::Continue(()),
                Err(error) => ControlFlow::Break(Err(error)),
            },
            Err(StoreError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::Interrupted =>
            {
                ControlFlow::Continue(())
            }
            Err(error) => ControlFlow::Break(Err(error)),
        }
    });
    match progress {
        ControlFlow::Break(result) => result,
        ControlFlow::Continue(()) => {
            Err(owner::refused("results spill write work budget exhausted"))
        }
    }
}

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.append(bytes) {
            Ok(()) => Ok(bytes.len()),
            Err(error) => {
                self.failure = Some(error);
                Err(std::io::Error::other("results spill write refused"))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.flush_buffer() {
            Ok(()) => Ok(()),
            Err(error) => {
                self.failure = Some(error);
                Err(std::io::Error::other("results spill flush refused"))
            }
        }
    }
}

#[cfg(all(test, unix))]
#[path = "writer_tests.rs"]
mod tests;
