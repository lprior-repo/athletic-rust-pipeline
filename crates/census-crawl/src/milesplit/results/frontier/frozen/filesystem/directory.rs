use super::{invariant, io_error, CrawlResult, File, Path};

pub(super) fn create(directory: &Path) -> CrawlResult<()> {
    let count = directory.ancestors().count();
    super::super::limit("request archive directory ancestors", count, 4096)?;
    let mut missing = Vec::new();
    missing
        .try_reserve_exact(count)
        .map_err(super::super::super::super::budget::reserve)?;
    directory
        .ancestors()
        .filter(|path| !path.as_os_str().is_empty())
        .try_for_each(|path| match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_dir() => Ok(()),
            Ok(_) => Err(invariant("request archive ancestor is not a directory")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(path);
                Ok(())
            }
            Err(source) => Err(io_error(path, source)),
        })?;
    std::fs::create_dir_all(directory).map_err(|source| io_error(directory, source))?;
    missing.iter().rev().try_for_each(|path| {
        sync(path)?;
        if let Some(parent) = path.parent() {
            sync(parent)?;
        }
        Ok(())
    })
}

fn sync(path: &Path) -> CrawlResult<()> {
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error(path, source))
}
