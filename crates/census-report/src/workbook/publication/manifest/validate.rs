use super::super::invariant;
use crate::report::{io_error, ReportResult};
use std::path::Path;

pub(super) fn validate_pointer(relative: &Path) -> ReportResult<()> {
    let mut components = relative.components();
    let valid = components.next()
        == Some(std::path::Component::Normal(std::ffi::OsStr::new(
            "generations",
        )))
        && components
            .next()
            .is_some_and(|part| matches!(part, std::path::Component::Normal(_)))
        && components.next().is_none();
    if !valid {
        return Err(invariant(
            "publication pointer escapes its generation directory".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_generation_paths(generations: &Path, directory: &Path) -> ReportResult<()> {
    for path in [generations, directory] {
        let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
        if !metadata.file_type().is_dir() {
            return Err(invariant(
                "publication generation must be a real directory".to_string(),
            ));
        }
    }
    Ok(())
}
