use std::path::Path;

use serde::Serialize;

use super::{classify, initialize, read_format, store_is_empty, StoreFormat, Verdict};
use super::{KEY_FORMAT_VERSION, STORE_SCHEMA_VERSION};
use crate::{Store, StoreError, StoreResult};

mod rewrite;

use rewrite::rewrite;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct MigrationReport {
    pub root: String,
    pub from_schema: Option<u32>,
    pub to_schema: u32,
    pub to_key_format: u32,
    pub rewritten_rows: u64,
    pub dropped_rows: u64,
    pub already_current: bool,
    pub integrity_ok: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct Rewritten {
    rewritten: u64,
    dropped: u64,
}

impl Store {
    pub fn migrate(root: impl AsRef<Path>) -> StoreResult<MigrationReport> {
        Self::migrate_with(root, super::DEFAULT_CACHE_BYTES)
    }

    pub fn migrate_with(root: impl AsRef<Path>, cache_bytes: u64) -> StoreResult<MigrationReport> {
        let root = root.as_ref().to_path_buf();
        crate::ensure_dirs(&root)?;
        let outcome: StoreResult<(StoreFormat, bool, Option<Rewritten>)> = {
            let (db, entities, journal, meta_keyspace, receipts) =
                crate::open_keyspaces(&root, cache_bytes)?;
            let format = read_format(&meta_keyspace)?;
            let already_current = matches!(classify(&format), Verdict::Current);
            let rewritten = match classify(&format) {
                Verdict::Current => None,
                Verdict::Newer {
                    what,
                    found,
                    supported,
                } => {
                    return Err(StoreError::FormatNewer {
                        what,
                        found,
                        supported,
                    })
                }
                Verdict::Unknown(detail) => return Err(StoreError::SchemaUnknown { detail }),
                Verdict::Migrate(_) => {
                    if store_is_empty(&entities, &journal, &receipts)? {
                        initialize(&db, &meta_keyspace)?;
                        None
                    } else {
                        Some(rewrite(&db, &entities, &meta_keyspace)?)
                    }
                }
            };
            Ok((format, already_current, rewritten))
        };
        let (format, already_current, rewritten) = outcome?;
        let store = Self::open_with(&root, cache_bytes)?;
        let integrity = store.integrity()?;
        Ok(MigrationReport {
            root: root.display().to_string(),
            from_schema: format.schema_version,
            to_schema: STORE_SCHEMA_VERSION,
            to_key_format: KEY_FORMAT_VERSION,
            rewritten_rows: rewritten.map_or(0, |rows| rows.rewritten),
            dropped_rows: rewritten.map_or(0, |rows| rows.dropped),
            already_current,
            integrity_ok: integrity.ok,
        })
    }
}
