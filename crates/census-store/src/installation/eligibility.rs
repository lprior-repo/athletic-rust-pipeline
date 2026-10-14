use census_domain::model::ProfileId;
use fjall::Readable;

use super::{HistoricalStateKind, InstallationError};
use crate::{Store, StoreError, StoreResult, Table};

const MAX_METADATA_ROWS: usize = 4096;

pub(super) fn require_empty(store: &Store) -> StoreResult<()> {
    if std::sync::Arc::strong_count(&store.appends) != 1 {
        return historical(HistoricalStateKind::SharedOwner);
    }
    if !store
        .entities
        .is_empty()
        .map_err(|source| StoreError::Read { source })?
    {
        return historical(HistoricalStateKind::Entities);
    }
    if !store
        .receipts
        .is_empty()
        .map_err(|source| StoreError::Read { source })?
    {
        return historical(HistoricalStateKind::Receipts);
    }
    require_empty_artifact_roots(store.root())?;
    require_empty_journal(store)?;
    require_empty_metadata(store)
}

fn require_empty_journal(store: &Store) -> StoreResult<()> {
    let snapshot = store.db.snapshot();
    crate::lineage::read(&snapshot, &store.journal)?;
    for guard in snapshot.prefix(&store.journal, b"").take(2) {
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        if key.as_ref() != Store::journal_key("export-lineage-v1", "store-identity").as_slice() {
            return historical(HistoricalStateKind::Journal);
        }
    }
    Ok(())
}

fn require_empty_metadata(store: &Store) -> StoreResult<()> {
    let format = store.store_format()?;
    if format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.lifecycle_revision != Some(2)
        || format.migration_target.is_some()
    {
        return historical(HistoricalStateKind::Metadata);
    }
    let snapshot = store.db.snapshot();
    for table in Table::ALL {
        for prefix in ["sequence", "rows"] {
            let key = format!("{prefix}:{}", table.file());
            let raw = snapshot
                .get(&store.meta, key.as_bytes())
                .map_err(|source| StoreError::Read { source })?
                .ok_or(InstallationError::HistoricalStatePresent {
                    kind: HistoricalStateKind::Metadata,
                })?;
            require_value(&raw, b"0")?;
        }
    }
    for (position, guard) in snapshot
        .prefix(&store.meta, b"")
        .take(MAX_METADATA_ROWS + 1)
        .enumerate()
    {
        if position == MAX_METADATA_ROWS {
            return Err(InstallationError::MetadataBudgetExceeded {
                max: MAX_METADATA_ROWS,
            }
            .into());
        }
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let raw = read_metadata_row(&snapshot, &store.meta, key.as_ref())?;
        let key =
            std::str::from_utf8(&key).map_err(|_| InstallationError::HistoricalStatePresent {
                kind: HistoricalStateKind::Metadata,
            })?;
        allowed_metadata(key, &raw)?;
    }
    Ok(())
}

fn read_metadata_row(
    snapshot: &fjall::Snapshot,
    meta: &fjall::Keyspace,
    key: &[u8],
) -> StoreResult<fjall::Slice> {
    let length = snapshot
        .size_of(meta, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| StoreError::Invariant {
            detail: "metadata row vanished from pinned snapshot".to_string(),
        })?;
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > super::MAX_INSTALLATION_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes,
            max: super::MAX_INSTALLATION_RECORD_BYTES,
        });
    }
    let raw = snapshot
        .get(meta, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| StoreError::Invariant {
            detail: "metadata row vanished from pinned snapshot".to_string(),
        })?;
    Ok(raw)
}

fn allowed_metadata(key: &str, raw: &[u8]) -> StoreResult<()> {
    match key {
        "schema:store_version"
        | "schema:key_format"
        | "schema:lifecycle_revision"
        | "schema:created_by"
        | "schema:created_at" => return Ok(()),
        "generation:entities" | "generation:derived:current" => return require_value(raw, b"0"),
        "generation:derived:next" | "generation:derived:reclaim_from" => {
            return require_value(raw, b"1")
        }
        "run-manifest" => return historical(HistoricalStateKind::LegacyRun),
        _ => {}
    }
    if let Some(id) = key.strip_prefix("profiles:v1:") {
        let id = ProfileId::parse(id).map_err(crate::ProfileRegistryError::InvalidPayload)?;
        crate::profiles::read::decode(&id, raw)?;
        return Ok(());
    }
    for table in Table::ALL {
        if key == format!("sequence:{}", table.file()) || key == format!("rows:{}", table.file()) {
            return require_value(raw, b"0");
        }
    }
    historical(HistoricalStateKind::Metadata)
}

fn require_value(actual: &[u8], expected: &[u8]) -> StoreResult<()> {
    if actual != expected {
        return historical(HistoricalStateKind::Generations);
    }
    Ok(())
}

fn historical(kind: HistoricalStateKind) -> StoreResult<()> {
    Err(InstallationError::HistoricalStatePresent { kind }.into())
}

fn require_empty_artifact_roots(root: &std::path::Path) -> StoreResult<()> {
    for name in ["http", "out", "entities", "journal", "capture", "captures"] {
        let path = root.join(name);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(StoreError::Io { path, source }),
        };
        if !metadata.file_type().is_dir() {
            return historical(HistoricalStateKind::Artifacts);
        }
        let mut entries = std::fs::read_dir(&path).map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })?;
        match entries.next() {
            None => {}
            Some(Ok(_)) => return historical(HistoricalStateKind::Artifacts),
            Some(Err(source)) => return Err(StoreError::Io { path, source }),
        }
    }
    Ok(())
}
