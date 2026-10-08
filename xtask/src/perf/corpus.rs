use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 100_000;
const MAX_PATH_BYTES: usize = 32 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 256 * 1024 * 1024;
const MAX_CORPUS_BYTES: u64 = 4 * 1024 * 1024 * 1024;

pub(super) struct Corpus {
    pub(super) lines: u64,
    pub(super) sha256: String,
}

pub(super) fn measure(root: &Path) -> Result<Corpus> {
    let mut files = inventory(root)?;
    files.sort_unstable();
    let mut hasher = Sha256::new();
    let mut lines = 0_u64;
    let mut bytes = 0_u64;
    for path in files {
        let relative = path
            .strip_prefix(root)?
            .to_str()
            .context("fixture path is not UTF-8")?;
        hasher.update(u64::try_from(relative.len())?.to_le_bytes());
        hasher.update(relative.as_bytes());
        let measured = measure_file(&path, &mut hasher)?;
        lines = lines
            .checked_add(measured.0)
            .context("fixture line count overflow")?;
        bytes = bytes
            .checked_add(measured.1)
            .context("fixture byte count overflow")?;
        if bytes > MAX_CORPUS_BYTES {
            bail!("fixture corpus exceeds four GiB");
        }
    }
    Ok(Corpus {
        lines,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

fn inventory(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = Vec::new();
    pending.try_reserve(1)?;
    pending.push(root.to_path_buf());
    let mut files = Vec::new();
    let mut admitted = (0usize, 0usize);
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            admit_path(&path, &mut admitted)?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                bail!("fixture corpus contains a symlink: {}", path.display());
            }
            if kind.is_dir() {
                append(&mut pending, path)?;
            } else if kind.is_file() {
                append(&mut files, path)?;
            } else {
                bail!(
                    "fixture corpus contains a non-regular entry: {}",
                    path.display()
                );
            }
        }
    }
    Ok(files)
}

fn admit_path(path: &Path, admitted: &mut (usize, usize)) -> Result<()> {
    admitted.0 = admitted
        .0
        .checked_add(1)
        .context("fixture entry count overflow")?;
    admitted.1 = admitted
        .1
        .checked_add(path.as_os_str().len())
        .context("fixture path bytes overflow")?;
    if admitted.0 > MAX_ENTRIES || admitted.1 > MAX_PATH_BYTES {
        bail!("fixture inventory admission capacity exhausted");
    }
    Ok(())
}

fn append(paths: &mut Vec<PathBuf>, path: PathBuf) -> Result<()> {
    paths
        .try_reserve(1)
        .context("allocating bounded fixture inventory")?;
    paths.push(path);
    Ok(())
}

fn measure_file(path: &Path, hasher: &mut Sha256) -> Result<(u64, u64)> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("opening fixture {}", path.display()))?;
    let expected = file.metadata()?.len();
    if expected > u64::try_from(MAX_FILE_BYTES)? {
        bail!("fixture exceeds 256 MiB: {}", path.display());
    }
    hasher.update(expected.to_le_bytes());
    let mut count = FileCount::default();
    let mut chunk = [0_u8; 32_768];
    let steps = MAX_FILE_BYTES
        .checked_div(chunk.len())
        .and_then(|steps| steps.checked_add(2))
        .context("fixture read bound overflow")?;
    for _ in 0..steps {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            return count.finish(path, expected);
        }
        let bytes = chunk
            .get(..read)
            .context("fixture read escaped its buffer")?;
        count.accept(bytes)?;
        hasher.update(bytes);
    }
    bail!("fixture read capacity exhausted: {}", path.display())
}

#[derive(Default)]
struct FileCount {
    bytes: u64,
    lines: u64,
    last: Option<u8>,
}

impl FileCount {
    fn accept(&mut self, bytes: &[u8]) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(bytes.len())?)
            .context("fixture byte count overflow")?;
        if self.bytes > u64::try_from(MAX_FILE_BYTES)? {
            bail!("fixture exceeds its read capacity");
        }
        self.lines = self
            .lines
            .checked_add(u64::try_from(
                bytes.iter().filter(|byte| **byte == b'\n').count(),
            )?)
            .context("fixture line count overflow")?;
        self.last = bytes.last().copied();
        Ok(())
    }

    fn finish(self, path: &Path, expected: u64) -> Result<(u64, u64)> {
        if self.bytes != expected {
            bail!("fixture changed length while measuring: {}", path.display());
        }
        let text = path.extension().is_some_and(|extension| {
            extension == "txt" || extension == "htm" || extension == "html"
        });
        let lines = self
            .lines
            .checked_add(u64::from(self.last.is_some_and(|last| last != b'\n')))
            .context("fixture line count overflow")?;
        Ok((if text { lines } else { 0 }, self.bytes))
    }
}
