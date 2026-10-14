use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use census_domain::{model::SourceMeetRef, UsJurisdiction};
use census_store::{StoreError, StoreResult};

use super::check_cancel;
use crate::restate_services::results_arms::budget::{
    self, DiskLease, ScratchPermit, SpillAdmission,
};

mod owner;
mod reader;
mod wire;
mod writer;

pub(in crate::restate_services::results_arms) struct Dataset {
    partitions: Vec<Partition>,
    _lease: DiskLease,
    admission: Arc<SpillAdmission>,
    _metadata: ScratchPermit,
    pub(in crate::restate_services::results_arms) disk_bytes: u64,
    pub(in crate::restate_services::results_arms) retained_bytes: u64,
}

struct Partition {
    file: File,
    path: PathBuf,
    jurisdiction: UsJurisdiction,
    bytes: u64,
    rows: u64,
    digest: [u8; 32],
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services::results_arms) struct RowCharge {
    pub(in crate::restate_services::results_arms) meet_heap: u64,
    pub(in crate::restate_services::results_arms) pending_heap: u64,
}

#[derive(Debug, Default)]
pub(in crate::restate_services::results_arms) struct Visits {
    pub(in crate::restate_services::results_arms) rows: u64,
    pub(in crate::restate_services::results_arms) bytes: u64,
    pub(in crate::restate_services::results_arms) read_calls: u64,
}

pub(in crate::restate_services::results_arms) struct Build {
    writers: Vec<writer::Writer>,
    directory: Option<owner::Directory>,
    lease: DiskLease,
    admission: Arc<SpillAdmission>,
    metadata: ScratchPermit,
}

impl Build {
    pub(in crate::restate_services::results_arms) fn new(
        root: &Path,
        admission: Arc<SpillAdmission>,
    ) -> StoreResult<Self> {
        let lease = admission.claim()?;
        let mut metadata = ScratchPermit::new(Arc::clone(&admission));
        metadata.reserve(budget::SPILL_METADATA_BYTES)?;
        let mut directory = owner::Directory::create(root, Arc::clone(&admission))?;
        let mut writers = Vec::new();
        let result = writers
            .try_reserve_exact(budget::SPILL_FILES)
            .map_err(|_| budget::allocation())
            .and_then(|()| {
                UsJurisdiction::ALL
                    .into_iter()
                    .try_for_each(|jurisdiction| {
                        writers.push(writer::Writer::new(
                            &mut directory,
                            jurisdiction,
                            Arc::clone(&admission),
                        )?);
                        Ok::<_, StoreError>(())
                    })
            });
        if let Err(primary) = result {
            drop(writers);
            return Err(directory.abort(primary));
        }
        Ok(Self {
            writers,
            directory: Some(directory),
            lease,
            admission,
            metadata,
        })
    }

    pub(in crate::restate_services::results_arms) fn push(
        &mut self,
        row: SourceMeetRef,
        cancel: &AtomicBool,
    ) -> StoreResult<()> {
        check_cancel(cancel)?;
        self.writers
            .iter_mut()
            .find(|writer| writer.jurisdiction() == row.jurisdiction)
            .ok_or_else(budget::allocation)?
            .push(&row)
    }

    pub(in crate::restate_services::results_arms) fn abort(
        mut self,
        primary: StoreError,
    ) -> StoreError {
        self.writers.clear();
        match self.directory.take() {
            Some(directory) => directory.abort(primary),
            None => primary,
        }
    }

    pub(in crate::restate_services::results_arms) fn finish(
        mut self,
        cancel: &AtomicBool,
    ) -> StoreResult<Dataset> {
        let partitions = match seal(std::mem::take(&mut self.writers), cancel) {
            Ok(partitions) => partitions,
            Err(primary) => return Err(self.abort(primary)),
        };
        let metadata = partitions
            .iter()
            .try_fold((0_u64, 0_u64), |(bytes, paths), partition| {
                self.directory
                    .as_mut()
                    .ok_or_else(budget::allocation)?
                    .seal_name(&partition.path, &partition.file, partition.bytes)?;
                let bytes = bytes
                    .checked_add(partition.bytes)
                    .ok_or(StoreError::CounterOverflow)?;
                let paths = paths
                    .checked_add(
                        u64::try_from(partition.path.capacity())
                            .map_err(|_| StoreError::CounterOverflow)?,
                    )
                    .ok_or(StoreError::CounterOverflow)?;
                Ok((bytes, paths))
            })
            .and_then(|(disk_bytes, paths)| {
                let retained_bytes = budget::slots::<Partition>(partitions.capacity())?
                    .checked_add(budget::slots::<Dataset>(1)?)
                    .and_then(|bytes| bytes.checked_add(paths))
                    .ok_or(StoreError::CounterOverflow)?;
                Ok((disk_bytes, retained_bytes))
            });
        let (disk_bytes, retained_bytes) = match metadata {
            Ok(metadata) => metadata,
            Err(primary) => {
                drop(partitions);
                return Err(self.abort(primary));
            }
        };
        let directory = match self.directory.take() {
            Some(directory) => directory,
            None => {
                drop(partitions);
                return Err(budget::allocation());
            }
        };
        if let Err(primary) = directory.unlink() {
            drop(partitions);
            return Err(primary);
        }
        let Self {
            writers: _,
            directory: _,
            lease,
            admission,
            metadata,
        } = self;
        Ok(Dataset {
            partitions,
            _lease: lease,
            admission,
            _metadata: metadata,
            disk_bytes,
            retained_bytes,
        })
    }
}

impl Dataset {
    pub(in crate::restate_services::results_arms) fn visit(
        &self,
        jurisdiction: UsJurisdiction,
        cancel: &AtomicBool,
        visitor: impl FnMut(&SourceMeetRef, RowCharge) -> StoreResult<()>,
    ) -> StoreResult<Visits> {
        let partition = self
            .partitions
            .iter()
            .find(|partition| partition.jurisdiction == jurisdiction)
            .ok_or_else(budget::allocation)?;
        reader::visit(partition, Arc::clone(&self.admission), cancel, visitor)
    }

    pub(in crate::restate_services::results_arms) fn writes(&self) -> (u64, u64) {
        self.admission.writes()
    }

    pub(in crate::restate_services::results_arms) fn peak_scratch(&self) -> u64 {
        self.admission.peak_scratch()
    }
}

fn seal(writers: Vec<writer::Writer>, cancel: &AtomicBool) -> StoreResult<Vec<Partition>> {
    let mut partitions = Vec::new();
    partitions
        .try_reserve_exact(budget::SPILL_FILES)
        .map_err(|_| budget::allocation())?;
    writers.into_iter().try_for_each(|writer| {
        check_cancel(cancel)?;
        partitions.push(writer.finish()?);
        Ok::<_, StoreError>(())
    })?;
    Ok(partitions)
}

#[cfg(all(test, unix))]
#[path = "spill_tests.rs"]
mod tests;
