use crate::{Store, StoreError, StoreResult};

pub fn temp_store() -> StoreResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir().map_err(|source| StoreError::Io {
        path: std::env::temp_dir(),
        source,
    })?;
    let store = Store::open(directory.path())?;
    Ok((directory, store))
}
