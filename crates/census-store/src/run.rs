use census_domain::model::RunManifest;
use fjall::{Keyspace, PersistMode, Readable, Snapshot};

use super::{Store, StoreError, StoreResult};
use crate::records::codec::{encode, MAX_RECORD_BYTES};

pub(crate) const RUN_MANIFEST: &str = "run-manifest";

pub(crate) fn read(snapshot: &Snapshot, meta: &Keyspace) -> StoreResult<Option<RunManifest>> {
    if let Some(length) = snapshot
        .size_of(meta, RUN_MANIFEST)
        .map_err(|source| StoreError::Read { source })?
    {
        let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if bytes > MAX_RECORD_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes,
                max: MAX_RECORD_BYTES,
            });
        }
    }
    let Some(raw) = snapshot
        .get(meta, RUN_MANIFEST)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    if raw.len() > MAX_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max: MAX_RECORD_BYTES,
        });
    }
    let text = std::str::from_utf8(&raw).map_err(|_| StoreError::SchemaUnknown {
        detail: format!("{RUN_MANIFEST} is not utf-8"),
    })?;
    serde_json::from_str(text)
        .map(Some)
        .map_err(|source| StoreError::Json {
            detail: format!("{RUN_MANIFEST} is not a census run manifest"),
            source,
        })
}

pub(super) fn write(store: &Store, manifest: &RunManifest) -> StoreResult<()> {
    let _binding = store.lock_appends();
    let capture = crate::acquisition::qualification::declaration(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        &store.store_format()?,
    )?;
    if let Some(record) = capture {
        if manifest.store_identity != record.store_identity().as_str()
            || manifest.cohort.get() != 2027
            || manifest.jurisdictions.is_empty()
            || manifest
                .jurisdictions
                .iter()
                .any(|j| !census_domain::UsJurisdiction::CENSUS_SCOPE.contains(j))
        {
            return Err(crate::CaptureError::CaptureRunMismatch.into());
        }
    }
    let state = crate::installation::validate_format(
        &store.db.snapshot(),
        &store.meta,
        &store.journal,
        store.store_format()?.lifecycle_revision,
    )?;
    crate::installation::require_legacy_execution(&state)?;
    if let Some(retained) = read(&store.db.snapshot(), &store.meta)? {
        if retained == *manifest {
            return Ok(());
        }
        return Err(StoreError::Refused {
            detail: format!(
                "this store is bound to census run {}-{} ({} jurisdictions, cohort {}) and cannot carry run {}-{} ({} jurisdictions, cohort {}): a fresh census needs its own store root",
                retained.run.season().get(),
                retained.run.revision(),
                retained.jurisdictions.len(),
                retained.cohort,
                manifest.run.season().get(),
                manifest.run.revision(),
                manifest.jurisdictions.len(),
                manifest.cohort
            ),
        });
    }
    let bytes = encode(manifest)?;
    let mut batch = store.db.batch();
    batch.insert(&store.meta, RUN_MANIFEST, bytes);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}

#[cfg(test)]
#[path = "run/tests.rs"]
mod tests;
