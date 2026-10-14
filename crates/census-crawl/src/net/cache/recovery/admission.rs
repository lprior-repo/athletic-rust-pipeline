use super::{inventory, reservations, shell};
use crate::net::FetchError;
use census_store::RecoveryArchiveIo;
use std::path::{Path, PathBuf};

pub(super) struct Admission<'a> {
    root: &'a Path,
    archive: &'a Path,
    reservations: &'a Path,
    capacity: u64,
}

impl<'a> Admission<'a> {
    pub(super) fn begin(
        root: &'a Path,
        paths: &'a super::paths::ArchivePaths,
        capacity: u64,
        io: &mut RecoveryArchiveIo<'_>,
    ) -> Result<Self, FetchError> {
        inventory::used(root, &paths.archive, io)?;
        reservations::recover(&paths.reservations, io)?;
        Ok(Self {
            root,
            archive: &paths.archive,
            reservations: &paths.reservations,
            capacity,
        })
    }

    pub(super) fn reserve(
        &self,
        body_digest: &str,
        metadata_digest: &str,
        bytes: (u64, u64),
        io: &mut RecoveryArchiveIo<'_>,
    ) -> Result<PathBuf, FetchError> {
        let path = reservations::path(self.reservations, body_digest, metadata_digest, io)?;
        let additional = bytes
            .0
            .checked_add(bytes.1)
            .ok_or_else(|| shell::limit("archive reservation"))?;
        if shell::regular(&path, io)?.is_some() {
            if reservations::read(&path, io)? != bytes {
                return Err(shell::invalid(&path, "archive reservation differs", io)?);
            }
            return Ok(path);
        }
        if additional != 0 {
            self.check(additional, io)?;
            reservations::write(&path, bytes, io)?;
        }
        Ok(path)
    }

    fn check(&self, additional: u64, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
        let used = inventory::used(self.root, self.archive, io)?;
        let (reserved, records) = reservations::scan(self.reservations, io)?;
        reservations::release(records, io)?;
        let Some(projected) =
            super::super::archive::admission::capacity_projection(used, reserved, additional)
        else {
            return Err(shell::invalid(
                self.archive,
                "archive capacity accounting overflowed",
                io,
            )?);
        };
        if projected > self.capacity {
            io.reserve_owned(self.archive.as_os_str().len())?;
            return Err(FetchError::ArchiveCapacity {
                archive: self.archive.to_path_buf(),
                used,
                requested: additional,
                capacity: self.capacity,
            });
        }
        Ok(())
    }

    pub(super) fn stage_overlap(
        &self,
        bytes: u64,
        io: &mut RecoveryArchiveIo<'_>,
    ) -> Result<(), FetchError> {
        self.check(bytes, io)
    }
}
