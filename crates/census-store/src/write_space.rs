use crate::{Store, StoreError, StoreResult};
use rustix::fs::StatVfs;
use std::io;
use std::path::Path;

const MIN_FILESYSTEM_RESERVE_BYTES: u64 = 64 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteSpaceEstimateV1 {
    required_bytes: u64,
}

impl WriteSpaceEstimateV1 {
    pub fn new(
        additional_write_bytes: u64,
        temporary_peak_bytes: u64,
        compaction_allowance_bytes: u64,
    ) -> StoreResult<Self> {
        let required_bytes = additional_write_bytes
            .checked_add(temporary_peak_bytes)
            .and_then(|bytes| bytes.checked_add(compaction_allowance_bytes))
            .and_then(|bytes| bytes.checked_add(MIN_FILESYSTEM_RESERVE_BYTES))
            .ok_or(StoreError::CounterExhausted)?;
        Ok(Self { required_bytes })
    }

    pub fn required_bytes(&self) -> u64 {
        self.required_bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WriteSpaceAdmissionV1 {
    available_bytes: u64,
    required_bytes: u64,
}

impl WriteSpaceAdmissionV1 {
    pub fn available_bytes(&self) -> u64 {
        self.available_bytes
    }

    pub fn required_bytes(&self) -> u64 {
        self.required_bytes
    }
}

impl Store {
    pub fn require_write_space(
        &self,
        estimate: &WriteSpaceEstimateV1,
    ) -> StoreResult<WriteSpaceAdmissionV1> {
        require_at_with(&self.root().join(crate::DB_DIR), estimate, |path| {
            rustix::fs::statvfs(path).map_err(io::Error::from)
        })
    }
}

fn require_at_with(
    path: &Path,
    estimate: &WriteSpaceEstimateV1,
    probe: impl FnOnce(&Path) -> io::Result<StatVfs>,
) -> StoreResult<WriteSpaceAdmissionV1> {
    let sample = probe(path).map_err(|source| StoreError::Io {
        path: path.to_owned(),
        source,
    })?;
    if sample.f_frsize == 0 {
        return Err(StoreError::Io {
            path: path.to_owned(),
            source: io::Error::new(
                io::ErrorKind::InvalidData,
                "filesystem fragment size is zero",
            ),
        });
    }
    let available = sample
        .f_bavail
        .checked_mul(sample.f_frsize)
        .ok_or(StoreError::CounterExhausted)?;
    admit_available(estimate, available)
}

fn admit_available(
    estimate: &WriteSpaceEstimateV1,
    available: u64,
) -> StoreResult<WriteSpaceAdmissionV1> {
    let needed = estimate.required_bytes();
    if available < needed {
        return Err(StoreError::DiskBudgetExceeded { needed, available });
    }
    Ok(WriteSpaceAdmissionV1 {
        available_bytes: available,
        required_bytes: needed,
    })
}

#[cfg(test)]
mod tests;
