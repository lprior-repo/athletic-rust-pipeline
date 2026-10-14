use std::path::Path;

use fjall::Database;

use crate::{Store, StoreError, StoreResult};

#[derive(Clone, Copy)]
pub(crate) enum OpenPurpose {
    LegacyExecution,
    InstallationControl,
    CaptureControl,
    CaptureExecution(census_domain::model::CaptureProtocolVersion),
}

impl Store {
    pub fn open_existing(root: impl AsRef<Path>) -> StoreResult<Self> {
        existing(
            root.as_ref(),
            OpenPurpose::LegacyExecution,
            super::DEFAULT_CACHE_BYTES,
        )
    }

    pub fn installation_control_existing(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::InstallationControl> {
        existing(
            root.as_ref(),
            OpenPurpose::InstallationControl,
            super::DEFAULT_CACHE_BYTES,
        )
        .map(crate::InstallationControl::new)
    }
    pub fn capture_qualification_control_existing(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::CaptureQualificationControl> {
        existing(
            root.as_ref(),
            OpenPurpose::CaptureControl,
            super::DEFAULT_CACHE_BYTES,
        )
        .map(crate::CaptureQualificationControl::new)
    }
    pub fn open_capture_qualification_v1(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::CaptureQualificationStore> {
        let store = existing(
            root.as_ref(),
            OpenPurpose::CaptureExecution(census_domain::model::CaptureProtocolVersion::V1),
            super::DEFAULT_CACHE_BYTES,
        )?;
        let record = crate::acquisition::qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )?
        .ok_or(crate::CaptureError::CaptureCapabilityMissing)?;
        Ok(crate::CaptureQualificationStore::new(store, record))
    }
    pub fn open_capture_qualification_v2(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::CaptureQualificationStore> {
        let store = existing(
            root.as_ref(),
            OpenPurpose::CaptureExecution(census_domain::model::CaptureProtocolVersion::V2),
            super::DEFAULT_CACHE_BYTES,
        )?;
        let record = crate::acquisition::qualification::declaration(
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.store_format()?,
        )?
        .ok_or(crate::CaptureError::CaptureCapabilityMissing)?;
        Ok(crate::CaptureQualificationStore::new(store, record))
    }
}

pub(crate) fn existing(root: &Path, purpose: OpenPurpose, cache_bytes: u64) -> StoreResult<Store> {
    existing_marker(root)?;
    let db = Database::builder(root.join(crate::DB_DIR))
        .cache_size(cache_bytes)
        .open()
        .map_err(|source| StoreError::Open { source })?;
    require_keyspaces(&db)?;
    let (db, entities, journal, meta, receipts) = crate::keyspaces(db)?;
    let format = super::require_current(root, super::read_format(&meta)?)?;
    let capture = crate::acquisition::validate_format(
        root,
        &db.snapshot(),
        &meta,
        &journal,
        &receipts,
        &format,
    )?;
    match purpose {
        OpenPurpose::LegacyExecution | OpenPurpose::InstallationControl => {
            crate::acquisition::require_legacy(&capture)?
        }
        OpenPurpose::CaptureControl | OpenPurpose::CaptureExecution(_) if capture.is_none() => {
            return Err(crate::CaptureError::CaptureCapabilityMissing.into())
        }
        OpenPurpose::CaptureExecution(expected)
            if capture
                .as_ref()
                .is_none_or(|record| record.protocol() != expected) =>
        {
            return Err(crate::CaptureError::CaptureProtocolUnsupported.into());
        }
        _ => {}
    }
    let keyspaces = ExistingKeyspaces {
        db,
        entities,
        journal,
        meta,
        receipts,
    };
    finish_existing(root, purpose, keyspaces, format)
}

pub(crate) fn require_keyspaces(db: &Database) -> StoreResult<()> {
    for name in [
        crate::ENTITIES,
        crate::JOURNAL,
        crate::META,
        crate::RECEIPTS,
    ] {
        if !db.keyspace_exists(name) {
            return Err(StoreError::Refused {
                detail: format!("existing census store is missing keyspace {name}"),
            });
        }
    }
    Ok(())
}

struct ExistingKeyspaces {
    db: Database,
    entities: fjall::Keyspace,
    journal: fjall::Keyspace,
    meta: fjall::Keyspace,
    receipts: fjall::Keyspace,
}

fn finish_existing(
    root: &Path,
    purpose: OpenPurpose,
    keyspaces: ExistingKeyspaces,
    format: crate::StoreFormat,
) -> StoreResult<Store> {
    let ExistingKeyspaces {
        db,
        entities,
        journal,
        meta,
        receipts,
    } = keyspaces;
    let state = crate::installation::validate_format(
        &db.snapshot(),
        &meta,
        &journal,
        format.lifecycle_revision,
    )?;
    match purpose {
        OpenPurpose::LegacyExecution | OpenPurpose::CaptureExecution(_) => {
            crate::installation::require_legacy_execution(&state)?
        }
        OpenPurpose::InstallationControl | OpenPurpose::CaptureControl => {}
    }
    match purpose {
        OpenPurpose::LegacyExecution | OpenPurpose::CaptureExecution(_) => {
            crate::ensure_dirs(root)?;
            Store::recover_opened(root.to_path_buf(), db, entities, journal, meta, receipts)
        }
        OpenPurpose::InstallationControl | OpenPurpose::CaptureControl => {
            control_store(root, db, entities, journal, meta, receipts)
        }
    }
}

fn control_store(
    root: &Path,
    db: Database,
    entities: fjall::Keyspace,
    journal: fjall::Keyspace,
    meta: fjall::Keyspace,
    receipts: fjall::Keyspace,
) -> StoreResult<Store> {
    let generations = crate::generation::Generations::seeded(&meta)?;
    let sequences = crate::sequences::Counters::retained(&meta)?;
    Ok(Store {
        root: root.to_path_buf(),
        db,
        entities,
        journal,
        meta,
        receipts,
        sequences: std::sync::Arc::new(sequences),
        generations: std::sync::Arc::new(generations),
        appends: std::sync::Arc::new(std::sync::Mutex::new(())),
        staging: std::sync::Arc::new(std::sync::Mutex::new(())),
    })
}

fn existing_marker(root: &Path) -> StoreResult<()> {
    let path = root.join(crate::DB_DIR).join("version");
    let metadata = std::fs::metadata(&path).map_err(|source| StoreError::Io {
        path: path.clone(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(StoreError::Refused {
            detail: format!(
                "existing Fjall marker {} is not a regular file",
                path.display()
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
