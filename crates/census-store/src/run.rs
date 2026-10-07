use census_domain::model::RunManifest;
use fjall::PersistMode;

use super::{meta, Store, StoreError, StoreResult};

const RUN_MANIFEST: &str = "run-manifest";

pub(super) fn read(store: &Store) -> StoreResult<Option<RunManifest>> {
    let Some(text) = meta::get_text(&store.meta, RUN_MANIFEST)? else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|source| StoreError::Json {
            detail: format!("{RUN_MANIFEST} is not a census run manifest"),
            source,
        })
}

pub(super) fn write(store: &Store, manifest: &RunManifest) -> StoreResult<()> {
    if let Some(retained) = read(store)? {
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
    let text = serde_json::to_string(manifest).map_err(|source| StoreError::Json {
        detail: format!("encoding {RUN_MANIFEST}"),
        source,
    })?;
    let mut batch = store.db.batch();
    meta::put_text(&mut batch, &store.meta, RUN_MANIFEST, &text);
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })
}
