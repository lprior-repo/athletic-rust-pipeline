use std::path::Path;

use super::super::{StoreError, StoreResult};

pub(crate) fn directory_bytes(root: &Path) -> StoreResult<u64> {
    let entries = std::fs::read_dir(root).map_err(|source| StoreError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    let mut total = 0_u64;
    for entry in entries {
        let entry = entry.map_err(|source| StoreError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|source| StoreError::Io {
            path: path.clone(),
            source,
        })?;
        let bytes = if metadata.is_dir() {
            directory_bytes(&path)?
        } else {
            metadata.len()
        };
        total = total.saturating_add(bytes);
    }
    Ok(total)
}
