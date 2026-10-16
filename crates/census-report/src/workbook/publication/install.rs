use super::{io_error, manifest, sync_directory, ReportResult};
use std::path::Path;

pub(super) fn finalize(directory: &Path) -> ReportResult<()> {
    manifest::verify_directory(directory)?;
    let mut entries = std::fs::read_dir(directory).map_err(|source| io_error(directory, source))?;
    entries.try_for_each(|entry| {
        let entry = entry.map_err(|source| io_error(directory, source))?;
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|source| io_error(&path, source))?;
        if entry.file_name() == "frozen-input.json.proofs" && metadata.file_type().is_dir() {
            return sync_directory(&path);
        }
        if !metadata.file_type().is_file() {
            return Err(super::invariant(
                "prepared bundle contains a non-file artifact".into(),
            ));
        }
        std::fs::File::open(&path)
            .and_then(|file| file.sync_all())
            .map_err(|source| io_error(&path, source))
    })?;
    sync_directory(directory)
}

pub(super) fn retain(stage: &Path, target: &Path, digest: &str) -> ReportResult<()> {
    match rustix::fs::renameat_with(
        rustix::fs::CWD,
        stage,
        rustix::fs::CWD,
        target,
        rustix::fs::RenameFlags::NOREPLACE,
    ) {
        Ok(()) => Ok(()),
        Err(rustix::io::Errno::EXIST) => {
            let retained = manifest::verify_directory(target)?;
            if retained.generation_digest != digest {
                return Err(super::invariant("publication generation collision".into()));
            }
            std::fs::remove_dir_all(stage).map_err(|source| io_error(stage, source))
        }
        Err(source) => Err(io_error(target, source.into())),
    }
}
