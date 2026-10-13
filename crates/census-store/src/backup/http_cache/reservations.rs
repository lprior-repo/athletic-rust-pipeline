use super::super::errors::io_err;
use std::path::Path;

pub(super) fn check(directory: &Path, issues: &mut Vec<String>) -> crate::StoreResult<()> {
    let mut entries = std::fs::read_dir(directory).map_err(|source| io_err(directory, source))?;
    if entries
        .next()
        .transpose()
        .map_err(|source| io_err(directory, source))?
        .is_some()
    {
        issues.push("archive/reservations: unresolved reservations remain".to_string());
    }
    Ok(())
}
