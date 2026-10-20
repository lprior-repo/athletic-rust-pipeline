use std::path::Path;

use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;
use fjall::Database;

mod capture;
mod marker;
mod public_identity;

use crate::{Store, StoreError, StoreResult};

#[derive(Clone, Copy)]
pub(crate) enum OpenPurpose {
    LegacyExecution,
    PublicIdentityExecution(PublicIdentityQualificationVersion),
    CurrentQualificationV3,
    PublicIdentityHistory,
    InstallationControl,
    CaptureControl,
    CaptureExecution(census_domain::model::CaptureProtocolVersion),
    Inspection,
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
    marker::require(root)?;
    let db = Database::builder(root.join(crate::DB_DIR))
        .cache_size(cache_bytes)
        .open()
        .map_err(|source| StoreError::Open { source })?;
    require_keyspaces(&db)?;
    let (db, entities, journal, meta, receipts) = crate::keyspaces(db)?;
    let format = super::require_current(root, super::read_format(&meta)?)?;
    validate_store_format(&db, &meta, &journal, &receipts, &format)?;
    let keyspaces = ExistingKeyspaces {
        db,
        entities,
        journal,
        meta,
        receipts,
    };
    keyspaces.validate_installation(purpose, &format)?;
    keyspaces.validate_identity(purpose, &format)?;
    let capture = crate::acquisition::validate_format(
        root,
        &keyspaces.db.snapshot(),
        &keyspaces.meta,
        &keyspaces.journal,
        &keyspaces.receipts,
        &format,
    )?;
    capture::validate_purpose(purpose, &capture)?;
    finish_existing(root, purpose, keyspaces)
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

fn validate_store_format(
    db: &Database,
    meta: &fjall::Keyspace,
    journal: &fjall::Keyspace,
    receipts: &fjall::Keyspace,
    format: &crate::StoreFormat,
) -> StoreResult<()> {
    super::validate_protected_authorities(&db.snapshot(), meta, journal, receipts, format)
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
) -> StoreResult<Store> {
    let ExistingKeyspaces {
        db,
        entities,
        journal,
        meta,
        receipts,
    } = keyspaces;
    match purpose {
        OpenPurpose::LegacyExecution
        | OpenPurpose::CaptureExecution(_)
        | OpenPurpose::CurrentQualificationV3
        | OpenPurpose::PublicIdentityExecution(_) => {
            crate::ensure_dirs(root)?;
            Store::recover_opened(root.to_path_buf(), db, entities, journal, meta, receipts)
        }
        OpenPurpose::InstallationControl
        | OpenPurpose::CaptureControl
        | OpenPurpose::PublicIdentityHistory
        | OpenPurpose::Inspection => control_store(root, db, entities, journal, meta, receipts),
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

#[cfg(test)]
mod tests;

impl ExistingKeyspaces {
    fn validate_installation(
        &self,
        purpose: OpenPurpose,
        format: &crate::StoreFormat,
    ) -> StoreResult<()> {
        let state = crate::installation::validate_format(
            &self.db.snapshot(),
            &self.meta,
            &self.journal,
            format.lifecycle_revision,
        )?;
        match purpose {
            OpenPurpose::LegacyExecution
            | OpenPurpose::CaptureExecution(_)
            | OpenPurpose::CurrentQualificationV3
            | OpenPurpose::PublicIdentityExecution(_) => {
                crate::installation::require_legacy_execution(&state)
            }
            OpenPurpose::InstallationControl
            | OpenPurpose::CaptureControl
            | OpenPurpose::PublicIdentityHistory
            | OpenPurpose::Inspection => Ok(()),
        }
    }

    fn validate_identity(
        &self,
        purpose: OpenPurpose,
        format: &crate::StoreFormat,
    ) -> StoreResult<()> {
        let identity = crate::public_identity::validate(
            &self.db.snapshot(),
            &self.entities,
            &self.meta,
            &self.journal,
            &self.receipts,
            format,
        )?;
        match purpose {
            OpenPurpose::Inspection => {}
            OpenPurpose::PublicIdentityExecution(expected) => identity
                .as_ref()
                .ok_or(crate::PublicIdentityError::OptInRequired)?
                .require_version(expected)?,
            OpenPurpose::CurrentQualificationV3 => identity
                .as_ref()
                .ok_or(crate::PublicIdentityError::OptInRequired)?
                .require_version(PublicIdentityQualificationVersion::RetainedAuthorityGraphV3)?,
            OpenPurpose::PublicIdentityHistory => {
                let version = identity
                    .as_ref()
                    .ok_or(crate::PublicIdentityError::OptInRequired)?
                    .declaration
                    .version();
                if version == PublicIdentityQualificationVersion::RetainedAuthorityGraphV3 {
                    return Err(crate::PublicIdentityError::Corrupt(
                        "current graph store is not historical",
                    )
                    .into());
                }
            }
            OpenPurpose::CaptureControl if identity.is_some() => {
                let capture = crate::acquisition::declaration(
                    &self.db.snapshot(),
                    &self.meta,
                    &self.journal,
                    format,
                )?;
                if capture.is_none() {
                    return Err(crate::PublicIdentityError::OptInRequired.into());
                }
                identity
                    .as_ref()
                    .ok_or(crate::PublicIdentityError::OptInRequired)?
                    .require_version(
                        PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
                    )?;
            }
            _ if identity.is_some() => return Err(crate::PublicIdentityError::OptInRequired.into()),
            _ => {}
        }
        Ok(())
    }
}
