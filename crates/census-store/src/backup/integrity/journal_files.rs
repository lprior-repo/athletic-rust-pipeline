use std::fs;

use super::{Store, StoreError};

pub(super) fn check(store: &Store, unreadable: &mut Vec<String>) -> super::StoreResult<()> {
    let journal_dir = store.root().join("journal");
    if !journal_dir.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(&journal_dir).map_err(|source| StoreError::Io {
        path: journal_dir.clone(),
        source,
    })?;
    for entry in entries {
        let path = entry
            .map_err(|source| StoreError::Io {
                path: journal_dir.clone(),
                source,
            })?
            .path();
        if !path.is_file() {
            continue;
        }
        if fs::File::open(&path).is_err() {
            let name = match path.file_name().map(|n| n.to_string_lossy().to_string()) {
                Some(value) => value,
                None => path.display().to_string(),
            };
            unreadable.push(name);
        }
    }
    Ok(())
}
