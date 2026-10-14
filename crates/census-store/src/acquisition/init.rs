use super::CaptureError;
use crate::{Store, StoreError, StoreResult, Table};
use census_domain::model::*;
use fjall::PersistMode;
use std::path::Path;

impl Store {
    pub fn create_capture_qualification_v1(
        root: impl AsRef<Path>,
    ) -> StoreResult<CaptureQualificationV1> {
        match create(root.as_ref(), CaptureProtocolVersion::V1)? {
            CaptureQualification::V1(record) => Ok(record),
            CaptureQualification::V2(_) => Err(CaptureError::CaptureProtocolUnsupported.into()),
        }
    }
    pub fn create_capture_qualification_v2(
        root: impl AsRef<Path>,
    ) -> StoreResult<CaptureQualificationV2> {
        match create(root.as_ref(), CaptureProtocolVersion::V2)? {
            CaptureQualification::V2(record) => Ok(record),
            CaptureQualification::V1(_) => Err(CaptureError::CaptureProtocolUnsupported.into()),
        }
    }
}
fn create(root: &Path, protocol: CaptureProtocolVersion) -> StoreResult<CaptureQualification> {
    claim_empty_root(root)?;
    let (db, entities, journal, meta, receipts) =
        crate::open_keyspaces(root, crate::DEFAULT_CACHE_BYTES)?;
    require_claimed_root(root)?;
    for keyspace in [&entities, &journal, &meta, &receipts] {
        if !keyspace
            .is_empty()
            .map_err(|source| StoreError::Read { source })?
        {
            return Err(CaptureError::CaptureInitExistingStore.into());
        }
    }
    let mut batch = db.batch();
    crate::format::stage_initialization(&mut batch, &meta, CAPTURE_QUALIFICATION_LIFECYCLE);
    for table in Table::ALL {
        crate::meta::put_text(
            &mut batch,
            &meta,
            &format!("sequence:{}", table.file()),
            "0",
        );
        crate::meta::put_text(&mut batch, &meta, &format!("rows:{}", table.file()), "0");
    }
    let identity = crate::lineage::stage_new(&mut batch, &journal, root)?;
    let record = match protocol {
        CaptureProtocolVersion::V1 => {
            CaptureQualification::V1(CaptureQualificationV1::new(identity))
        }
        CaptureProtocolVersion::V2 => {
            CaptureQualification::V2(CaptureQualificationV2::new(identity))
        }
    };
    batch.insert(
        &meta,
        protocol.qualification_key(),
        crate::records::codec::encode_canonical(&record)?,
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    crate::ensure_dirs(root)?;
    crate::fs::fsync_dir(root)?;
    let format = crate::format::read_format(&meta)?;
    let verified = super::qualification::validate_format(
        root,
        &db.snapshot(),
        &meta,
        &journal,
        &receipts,
        &format,
    )?
    .ok_or(CaptureError::CaptureCapabilityMissing)?;
    if verified != record {
        return Err(CaptureError::CaptureMarkerMismatch.into());
    }
    Ok(verified)
}
fn claim_empty_root(root: &Path) -> StoreResult<()> {
    match std::fs::symlink_metadata(root) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Err(CaptureError::CaptureInitRootNotEmpty.into());
            }
            let mut entries = std::fs::read_dir(root).map_err(|source| io(root, source))?;
            if let Some(entry) = entries.next() {
                entry.map_err(|source| io(root, source))?;
                return if root.join("fjall/version").is_file() {
                    Err(CaptureError::CaptureInitExistingStore.into())
                } else {
                    Err(CaptureError::CaptureInitRootNotEmpty.into())
                };
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::fs::create_dir_all_synced(root).map_err(|source| io(root, source))?
        }
        Err(source) => return Err(io(root, source)),
    }
    let fjall = root.join("fjall");
    std::fs::create_dir(&fjall).map_err(|source| {
        if source.kind() == std::io::ErrorKind::AlreadyExists {
            CaptureError::CaptureInitExistingStore.into()
        } else {
            io(&fjall, source)
        }
    })?;
    crate::fs::fsync_dir(root)
}
fn require_claimed_root(root: &Path) -> StoreResult<()> {
    let mut entries = std::fs::read_dir(root).map_err(|source| io(root, source))?;
    let first = entries
        .next()
        .transpose()
        .map_err(|source| io(root, source))?
        .ok_or(CaptureError::CaptureInitRootNotEmpty)?;
    if first.file_name() != "fjall"
        || entries
            .next()
            .transpose()
            .map_err(|source| io(root, source))?
            .is_some()
    {
        return Err(CaptureError::CaptureInitRootNotEmpty.into());
    }
    Ok(())
}
fn io(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}
