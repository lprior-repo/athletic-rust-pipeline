use super::*;
use rustix::fs::{Dir, FileType, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::io::Read;

const MAX_FILES: usize = 4096;
const MAX_DIRECTORIES: usize = 4096;
const MAX_BYTES: u64 = 256 * 1024 * 1024;
const MAX_WALK_STEPS: usize = 20480;
const BUFFER_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
pub(super) struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}

struct Frame {
    relative: PathBuf,
    entries: Dir,
}

pub(super) fn read(root: &Path) -> TestResult<Vec<Entry>> {
    let root = rustix::fs::open(root, directory_flags(), Mode::empty())?;
    let mut pending = Vec::new();
    pending.try_reserve_exact(MAX_DIRECTORIES)?;
    pending.push(Frame {
        relative: PathBuf::new(),
        entries: Dir::new(root)?,
    });
    let mut files = Vec::new();
    files.try_reserve_exact(MAX_FILES)?;
    let mut directories = 1usize;
    let mut bytes = 0u64;
    for _ in 0..MAX_WALK_STEPS {
        let Some(frame) = pending.last_mut() else {
            files.sort_unstable_by(|left: &Entry, right: &Entry| left.path.cmp(&right.path));
            if files.windows(2).any(|pair| {
                pair.first()
                    .zip(pair.get(1))
                    .is_some_and(|(a, b)| a.path == b.path)
            }) {
                return Err("handoff inventory contains duplicate paths".into());
            }
            return Ok(files);
        };
        let Some(entry) = frame.entries.next() else {
            drop(pending.pop());
            continue;
        };
        let entry = entry?;
        let name = entry.file_name().to_str()?;
        if name == "." || name == ".." {
            continue;
        }
        let relative = paths::relative(&frame.relative, name)?;
        let file = open_entry(&frame.entries, name)?;
        let metadata = file.metadata()?;
        if metadata.is_dir() {
            directories = directories
                .checked_add(1)
                .ok_or("handoff directory count overflow")?;
            if directories > MAX_DIRECTORIES {
                return Err("handoff directory bound exceeded".into());
            }
            pending.push(Frame {
                relative,
                entries: Dir::new(file)?,
            });
        } else if metadata.is_file() {
            if files.len() >= MAX_FILES {
                return Err("handoff file count bound exceeded".into());
            }
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or("handoff byte count overflow")?;
            if bytes > MAX_BYTES {
                return Err("handoff total file byte bound exceeded".into());
            }
            files.push(hash_file(file, relative, metadata.len())?);
        } else {
            return Err("handoff entry changed to a nonregular file".into());
        }
    }
    Err("handoff inventory traversal bound exceeded".into())
}

fn directory_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

fn open_entry(directory: &Dir, name: &str) -> TestResult<File> {
    let fd = directory.fd()?;
    let status = rustix::fs::statat(fd, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)?;
    let flags = entry_flags(FileType::from_raw_mode(status.st_mode))?;
    let opened = rustix::fs::openat(fd, name, flags, Mode::empty())?;
    let actual = rustix::fs::fstat(&opened)?;
    if actual.st_dev != status.st_dev
        || actual.st_ino != status.st_ino
        || actual.st_mode != status.st_mode
    {
        return Err("handoff inventory entry changed during opening".into());
    }
    Ok(File::from(opened))
}

pub(super) fn entry_flags(kind: FileType) -> TestResult<OFlags> {
    match kind {
        FileType::Directory => Ok(directory_flags()),
        FileType::RegularFile => {
            Ok(OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC)
        }
        _ => Err("handoff inventory rejects symlinks and nonregular files".into()),
    }
}

fn hash_file(mut file: File, relative: PathBuf, expected: u64) -> TestResult<Entry> {
    let mut hash = Sha256::new();
    let mut remaining = expected;
    let mut buffer = [0u8; BUFFER_BYTES];
    for _ in 0..=MAX_BYTES / u64::try_from(BUFFER_BYTES)? {
        if remaining == 0 {
            break;
        }
        let length = usize::try_from(remaining.min(u64::try_from(BUFFER_BYTES)?))?;
        let chunk = buffer.get_mut(..length).ok_or("handoff hash chunk bound")?;
        file.read_exact(chunk)?;
        hash.update(chunk);
        remaining = remaining
            .checked_sub(u64::try_from(length)?)
            .ok_or("handoff hash byte underflow")?;
    }
    if remaining != 0 || file.read(&mut [0u8; 1])? != 0 || file.metadata()?.len() != expected {
        return Err("handoff file size changed during hashing".into());
    }
    Ok(Entry {
        path: paths::text(&relative)?.to_owned(),
        bytes: expected,
        sha256: format!("{:x}", hash.finalize()),
    })
}
