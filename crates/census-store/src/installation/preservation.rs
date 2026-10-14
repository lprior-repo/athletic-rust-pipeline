use fjall::{Database, Keyspace, Readable};
use std::path::Path;

use crate::{StoreError, StoreResult};

pub(crate) fn validate_backup(
    root: &Path,
    db: &Database,
    meta: &Keyspace,
    journal: &Keyspace,
) -> StoreResult<()> {
    let snapshot = db.snapshot();
    let records = match snapshot.prefix(meta, b"installation:").next() {
        Some(guard) => {
            guard.key().map_err(|source| StoreError::Read { source })?;
            true
        }
        None => false,
    };
    let raw = snapshot
        .get(meta, crate::format::LIFECYCLE_REVISION_KEY)
        .map_err(|source| StoreError::Read { source })?;
    let lifecycle = raw
        .as_ref()
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.trim().parse::<u32>().ok());
    if !records && lifecycle != Some(super::PRODUCTION_LIFECYCLE) {
        return Ok(());
    }
    let format = crate::format::require_semantic_format(root, db, meta, journal)?;
    if format.schema_version != Some(crate::format::STORE_SCHEMA_VERSION)
        || format.key_format != Some(crate::format::KEY_FORMAT_VERSION)
    {
        return Err(StoreError::SchemaUnknown {
            detail: "installation backup requires schema 2/key format 1".to_string(),
        });
    }
    super::read::validate_format(&snapshot, meta, journal, format.lifecycle_revision)?;
    Ok(())
}
