use super::TestResult;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path};

const MAX_FILES: usize = 4096;
const MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct File {
    pub(super) path: String,
    pub(super) bytes: u64,
    pub(super) sha256: String,
}

pub(super) fn inventory(root: &Path) -> TestResult<Vec<File>> {
    check!(root.is_absolute());
    check!(root
        .components()
        .all(|component| matches!(component, Component::RootDir | Component::Normal(_))));
    check!(std::fs::symlink_metadata(root)?.file_type().is_dir());
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut directories = 1usize;
    let mut bytes = 0u64;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            let path = entry?.path();
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or("UTF-8 handoff path")?;
            valid_path(relative)?;
            let metadata = std::fs::symlink_metadata(&path)?;
            let kind = metadata.file_type();
            check!(!kind.is_symlink());
            if kind.is_dir() {
                directories = directories
                    .checked_add(1)
                    .ok_or("handoff directory count overflow")?;
                check!(directories <= MAX_FILES);
                pending.push(path);
            } else {
                check!(kind.is_file());
                check!(files.len() < MAX_FILES);
                bytes = bytes
                    .checked_add(metadata.len())
                    .ok_or("handoff byte count overflow")?;
                check!(bytes <= MAX_BYTES);
                files.push(File {
                    path: relative.into(),
                    bytes: metadata.len(),
                    sha256: hash(&path)?,
                });
            }
        }
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    check!(!files.is_empty());
    Ok(files)
}

pub(super) fn verify(root: &Path, expected: &[File]) -> TestResult {
    check!(expected.len() <= MAX_FILES);
    check!(expected
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left.path < right.path)));
    for file in expected {
        valid_path(&file.path)?;
        check!(hex(&file.sha256, 64));
    }
    check!(eq; inventory(root)?, expected);
    Ok(())
}

pub(super) fn copy(root: &Path, destination: &Path, expected: &[File]) -> TestResult {
    verify(root, expected)?;
    std::fs::create_dir(destination)?;
    for file in expected {
        let target = destination.join(&file.path);
        let parent = target.parent().ok_or("copy parent")?;
        std::fs::create_dir_all(parent)?;
        check!(eq; std::fs::copy(root.join(&file.path), &target)?, file.bytes);
    }
    verify(destination, expected)?;
    verify(root, expected)
}

pub(super) fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_path(value: &str) -> TestResult {
    check!(!value.is_empty());
    check!(value.len() <= 4096);
    check!(Path::new(value)
        .components()
        .all(|component| matches!(component, Component::Normal(_))));
    Ok(())
}

fn hash(path: &Path) -> TestResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65_536];
    let mut total = 0u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(count)?)
            .ok_or("handoff hash size overflow")?;
        check!(total <= MAX_BYTES);
        digest.update(buffer.get(..count).ok_or("handoff hash read boundary")?);
    }
    Ok(format!("{:x}", digest.finalize()))
}
