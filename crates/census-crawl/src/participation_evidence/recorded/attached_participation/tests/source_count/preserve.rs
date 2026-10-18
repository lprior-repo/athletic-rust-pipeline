use super::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
const MAX_FILES: usize = 4096;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

pub(in super::super) fn files(root: &Path) -> TestResult<BTreeMap<PathBuf, (u64, String)>> {
    let mut pending = vec![root.to_path_buf()];
    let mut discovered = 1usize;
    let mut total = 0u64;
    let mut output = BTreeMap::new();
    for _ in 0..MAX_FILES {
        let Some(path) = pending.pop() else {
            return Ok(output);
        };
        let metadata = std::fs::symlink_metadata(&path)?;
        check!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                discovered = discovered
                    .checked_add(1)
                    .ok_or("inventory count overflow")?;
                check!(discovered <= MAX_FILES);
                pending.push(entry?.path());
            }
        } else {
            check!(metadata.is_file());
            total = total
                .checked_add(metadata.len())
                .ok_or("inventory bytes overflow")?;
            check!(total <= MAX_BYTES);
            output.insert(
                path.strip_prefix(root)?.to_path_buf(),
                fingerprint(&path, metadata.len())?,
            );
        }
    }
    check!(pending.is_empty());
    Ok(output)
}

fn fingerprint(path: &Path, expected: u64) -> TestResult<(u64, String)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; CHUNK];
    let mut hash = Sha256::new();
    let mut read = 0u64;
    for _ in 0..8193 {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            check!(eq; read, expected);
            return Ok((read, format!("{:x}", hash.finalize())));
        }
        read = read
            .checked_add(u64::try_from(count)?)
            .ok_or("read bytes overflow")?;
        check!(read <= expected && read <= MAX_BYTES);
        hash.update(buffer.get(..count).ok_or("bounded read slice")?);
    }
    Err("closed fixture hashing exceeds work bound".into())
}
