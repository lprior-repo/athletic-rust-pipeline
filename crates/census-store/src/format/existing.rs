use std::path::Path;

use fjall::Database;

use crate::{Store, StoreError, StoreResult};

impl Store {
    pub fn open_existing(root: impl AsRef<Path>) -> StoreResult<Self> {
        let root = root.as_ref();
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
        super::require_current(root, super::read_format(&meta)?)?;
        crate::ensure_dirs(root)?;
        Self::recover_opened(root.to_path_buf(), db, entities, journal, meta, receipts)
    }
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
