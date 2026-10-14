use std::path::Path;

use fjall::Database;

use crate::{Store, StoreError, StoreResult};

#[derive(Clone, Copy)]
enum OpenPurpose {
    LegacyExecution,
    InstallationControl,
}

impl Store {
    pub fn open_existing(root: impl AsRef<Path>) -> StoreResult<Self> {
        existing(root.as_ref(), OpenPurpose::LegacyExecution)
    }

    pub fn installation_control_existing(
        root: impl AsRef<Path>,
    ) -> StoreResult<crate::InstallationControl> {
        existing(root.as_ref(), OpenPurpose::InstallationControl)
            .map(crate::InstallationControl::new)
    }
}

fn existing(root: &Path, purpose: OpenPurpose) -> StoreResult<Store> {
    existing_marker(root)?;
    let db = Database::builder(root.join(crate::DB_DIR))
        .cache_size(super::DEFAULT_CACHE_BYTES)
        .open()
        .map_err(|source| StoreError::Open { source })?;
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
    let (db, entities, journal, meta, receipts) = crate::keyspaces(db)?;
    let format = super::require_semantic_format(root, &db, &meta, &journal)?;
    let state = crate::installation::validate_format(
        &db.snapshot(),
        &meta,
        &journal,
        format.lifecycle_revision,
    )?;
    match purpose {
        OpenPurpose::LegacyExecution => crate::installation::require_legacy_execution(&state)?,
        OpenPurpose::InstallationControl => {}
    }
    match purpose {
        OpenPurpose::LegacyExecution => {
            crate::ensure_dirs(root)?;
            Store::recover_opened(root.to_path_buf(), db, entities, journal, meta, receipts)
        }
        OpenPurpose::InstallationControl => {
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
