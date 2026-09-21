use super::SourceManifest;
use anyhow::{bail, Context, Result};
use sha2::{Digest as ShaDigest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub(super) fn detail_path(destination: &Path) -> PathBuf {
    destination.with_extension("jsonl")
}

pub(super) fn destination_parent(path: &Path) -> Result<PathBuf> {
    let parent = match path.parent().filter(|value| !value.as_os_str().is_empty()) {
        Some(parent) => parent,
        None => Path::new("."),
    };
    fs::canonicalize(parent).context("resolving export destination directory")
}

pub(super) fn reject_detail_destination(
    destination: &Path,
    detail: &Path,
    manifest: &SourceManifest,
) -> Result<()> {
    if destination == detail {
        bail!("XLSX and detail destinations must differ");
    }
    if fs::symlink_metadata(detail).is_ok() {
        bail!("detail export destination already exists or is a symlink");
    }
    let parent = destination_parent(detail)?;
    let name = detail
        .file_name()
        .context("detail export destination has no filename")?;
    let candidate = parent.join(name);
    [manifest.original.as_path(), manifest.frozen.as_path()]
        .iter()
        .try_for_each(|source| {
            if fs::canonicalize(source).is_ok_and(|value| value == candidate) {
                bail!("detail destination aliases source workbook");
            }
            Ok(())
        })
}

pub(super) fn persist_detail(temporary: NamedTempFile, path: &Path, parent: PathBuf) -> Result<()> {
    temporary
        .persist_noclobber(path)
        .map_err(|error| anyhow::Error::new(error.error))
        .context("publishing detail artifact without clobbering")?;
    File::open(parent)
        .context("opening export directory")?
        .sync_all()
        .context("syncing export directory")
}

pub(super) fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("opening artifact for hashing")?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    loop {
        let count = file
            .read(&mut buffer)
            .context("reading artifact for hashing")?;
        if count == 0 {
            break;
        }
        let chunk = buffer
            .get(..count)
            .context("read count exceeds hashing buffer")?;
        hash.update(chunk);
    }
    Ok(format!("{:x}", hash.finalize()))
}
