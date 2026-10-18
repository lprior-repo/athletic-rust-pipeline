use crate::{StoreError, StoreResult};
use std::path::Path;

pub(super) fn require(root: &Path) -> StoreResult<()> {
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
