use super::TestResult;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Component, Path};

const MAX_ENTRIES: usize = 4096;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(in super::super) struct File {
    path: String,
    bytes: u64,
    sha256: String,
}

pub(super) fn fresh_destination(path: &Path) -> TestResult {
    check!(path.is_absolute());
    check!(path
        .components()
        .all(|part| matches!(part, Component::RootDir | Component::Normal(_))));
    let parent = path.parent().ok_or("retention parent")?;
    check!(eq; std::fs::canonicalize(parent)?, parent);
    check!(std::fs::symlink_metadata(parent)?.is_dir());
    check!(
        matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    );
    disjoint(path, &std::env::current_dir()?)
}

pub(super) fn disjoint(destination: &Path, source: &Path) -> TestResult {
    let source = std::fs::canonicalize(source)?;
    check!(!destination.starts_with(&source));
    check!(!source.starts_with(destination));
    Ok(())
}

pub(in super::super) fn inventory(root: &Path) -> TestResult<Vec<File>> {
    check!(std::fs::symlink_metadata(root)?.file_type().is_dir());
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut entries = 0_usize;
    let mut total = 0_u64;
    for _ in 0..MAX_ENTRIES {
        let Some(directory) = pending.pop() else {
            break;
        };
        for entry in std::fs::read_dir(directory)?.take(MAX_ENTRIES + 1) {
            let path = entry?.path();
            entries = entries.checked_add(1).ok_or("retention entry overflow")?;
            check!(entries <= MAX_ENTRIES);
            let metadata = std::fs::symlink_metadata(&path)?;
            if metadata.file_type().is_dir() {
                pending.push(path);
            } else {
                check!(metadata.file_type().is_file());
                let bytes = read(&path)?;
                let length = u64::try_from(bytes.len())?;
                total = total.checked_add(length).ok_or("retention byte overflow")?;
                check!(total <= MAX_TOTAL_BYTES);
                let relative = path.strip_prefix(root)?;
                check!(relative
                    .components()
                    .all(|part| matches!(part, Component::Normal(_))));
                files.push(File {
                    path: relative.to_str().ok_or("retention UTF-8 path")?.to_owned(),
                    bytes: length,
                    sha256: digest(&bytes),
                });
            }
        }
    }
    check!(pending.is_empty());
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub(in super::super) fn read(path: &Path) -> TestResult<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    check!(metadata.file_type().is_file());
    check!(metadata.len() <= MAX_FILE_BYTES);
    let file = std::fs::File::open(path)?;
    check!(eq; file.metadata()?.len(), metadata.len());
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    check!(eq; u64::try_from(bytes.len())?, metadata.len());
    Ok(bytes)
}

pub(in super::super) fn copy(source: &Path, destination: &Path) -> TestResult<Vec<File>> {
    let before = inventory(source)?;
    check!(!before.is_empty());
    std::fs::create_dir(destination)?;
    for entry in &before {
        let bytes = read(&source.join(&entry.path))?;
        check!(eq; digest(&bytes), entry.sha256);
        let target = destination.join(&entry.path);
        std::fs::create_dir_all(target.parent().ok_or("retention copy parent")?)?;
        write(&target, &bytes)?;
    }
    check!(eq; inventory(source)?, before);
    check!(eq; inventory(destination)?, before);
    Ok(before)
}

pub(super) fn write(path: &Path, bytes: &[u8]) -> TestResult {
    check!(u64::try_from(bytes.len())? <= MAX_FILE_BYTES);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    check!(eq; read(path)?, bytes);
    Ok(())
}

pub(in super::super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(in super::super) fn revision(name: &str) -> TestResult<String> {
    let value = std::env::var(name)?;
    check!(
        value.len() == 40
            && value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    Ok(value)
}

#[test]
fn retained_copy_preserves_inventory_and_refuses_overwrite() -> TestResult {
    let source = tempfile::tempdir()?;
    let destination = tempfile::tempdir()?;
    std::fs::create_dir(source.path().join("proofs"))?;
    write(&source.path().join("archive.json"), b"retained archive")?;
    write(&source.path().join("proofs/leaf"), b"retained proof")?;
    let target = destination.path().join("copy");
    let expected = copy(source.path(), &target)?;
    check!(eq; inventory(&target)?, expected);
    check!(copy(source.path(), &target).is_err());
    check!(write(&target.join("archive.json"), b"replacement").is_err());
    std::fs::write(target.join("proofs/leaf"), b"changed")?;
    check!(inventory(&target)? != expected);
    Ok(())
}

#[test]
fn retained_destination_refuses_overlap_and_existing_paths() -> TestResult {
    let source = tempfile::tempdir()?;
    check!(fresh_destination(source.path()).is_err());
    check!(fresh_destination(Path::new("relative-retention")).is_err());
    check!(disjoint(&source.path().join("nested"), source.path()).is_err());
    check!(disjoint(
        source.path().parent().ok_or("temporary parent")?,
        source.path()
    )
    .is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn retained_inventory_refuses_symlink_leaf() -> TestResult {
    let source = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    write(&other.path().join("leaf"), b"external")?;
    std::os::unix::fs::symlink(other.path().join("leaf"), source.path().join("leaf"))?;
    check!(inventory(source.path()).is_err());
    Ok(())
}
