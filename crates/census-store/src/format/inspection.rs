use super::existing::{existing, OpenPurpose};
use crate::{IntegrityReport, Store, StoreResult};
use std::path::Path;

pub struct StoreIntegrityInspection {
    store: Store,
}

impl StoreIntegrityInspection {
    pub fn open_existing(root: impl AsRef<Path>) -> StoreResult<Self> {
        existing(
            root.as_ref(),
            OpenPurpose::Inspection,
            super::DEFAULT_CACHE_BYTES,
        )
        .map(|store| Self { store })
    }

    pub fn integrity(&self) -> StoreResult<IntegrityReport> {
        self.store.integrity()
    }

    pub fn http_cache_integrity(&self) -> StoreResult<Vec<String>> {
        self.store.http_cache_integrity()
    }
}
