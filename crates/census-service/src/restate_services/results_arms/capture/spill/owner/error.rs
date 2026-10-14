use std::path::Path;

use census_store::StoreError;

pub(in super::super) fn io_error(path: &Path, source: std::io::Error) -> StoreError {
    StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub(in super::super) fn refused(detail: &str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}
