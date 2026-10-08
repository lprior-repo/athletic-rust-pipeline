use super::{invariant, Archived, Manifest, BUFFER_BYTES, MAX_BYTES};
use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

mod directory;
mod reader;

pub(super) use reader::open;
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct HashWriter {
    hash: Sha256,
    bytes: usize,
}

impl Default for HashWriter {
    fn default() -> Self {
        Self {
            hash: Sha256::new(),
            bytes: 0,
        }
    }
}

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self.bytes.checked_add(bytes.len()).ok_or_else(exhausted)?;
        if self.bytes > MAX_BYTES {
            return Err(exhausted());
        }
        self.hash.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FileWriter {
    file: File,
    buffer: [u8; BUFFER_BYTES],
    used: usize,
    hash: HashWriter,
}

impl Write for FileWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.hash.write(bytes)?;
        bytes
            .chunks(BUFFER_BYTES)
            .try_for_each(|chunk| self.accept(chunk))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file
            .write_all(self.buffer.get(..self.used).ok_or_else(exhausted)?)?;
        self.used = 0;
        self.file.flush()
    }
}

impl FileWriter {
    fn accept(&mut self, bytes: &[u8]) -> io::Result<()> {
        let available = BUFFER_BYTES.checked_sub(self.used).ok_or_else(exhausted)?;
        if bytes.len() > available {
            self.flush()?;
        }
        let end = self.used.checked_add(bytes.len()).ok_or_else(exhausted)?;
        let held = self.buffer.get_mut(self.used..end).ok_or_else(exhausted)?;
        held.copy_from_slice(bytes);
        self.used = end;
        Ok(())
    }
}

pub(super) fn digest(value: &impl Serialize) -> CrawlResult<String> {
    let mut writer = HashWriter::default();
    encode(&mut writer, value)?;
    Ok(format!("{:x}", writer.hash.finalize()))
}

pub(super) fn archive(
    directory: &Path,
    input: &impl Serialize,
    generation_digest: String,
    request_count: usize,
) -> CrawlResult<Archived> {
    directory::create(directory)?;
    let (input_digest, bytes) = publish(directory, input, None)?;
    let manifest = Manifest {
        input_file: format!("{input_digest}.json"),
        input_digest,
        generation_digest,
        bytes,
        request_count,
    };
    let name = format!("{}.manifest.json", manifest.input_digest);
    publish(directory, &manifest, Some(&name))?;
    Ok(Archived {
        path: directory.join(name),
        manifest,
    })
}

fn publish(
    directory: &Path,
    value: &impl Serialize,
    name: Option<&str>,
) -> CrawlResult<(String, usize)> {
    let sequence = TEMPORARY_SEQUENCE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| invariant("immutable input temporary sequence exhausted"))?;
    let temporary = directory.join(format!(".pending.{}.{sequence}", std::process::id()));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| io_error(&temporary, source))?;
    let result = finish_file(directory, value, name, &temporary, file);
    std::fs::remove_file(&temporary).map_err(|source| io_error(&temporary, source))?;
    result
}

fn finish_file(
    directory: &Path,
    value: &impl Serialize,
    name: Option<&str>,
    temporary: &Path,
    file: File,
) -> CrawlResult<(String, usize)> {
    let mut writer = FileWriter {
        file,
        buffer: [0; BUFFER_BYTES],
        used: 0,
        hash: HashWriter::default(),
    };
    encode(&mut writer, value)?;
    writer
        .flush()
        .map_err(|source| io_error(temporary, source))?;
    writer
        .file
        .sync_all()
        .map_err(|source| io_error(temporary, source))?;
    freeze(&writer.file, temporary)?;
    let bytes = writer.hash.bytes;
    let digest = format!("{:x}", writer.hash.hash.finalize());
    let target = directory.join(name.map_or_else(|| format!("{digest}.json"), str::to_string));
    link(temporary, &target)?;
    verify(&target, &digest, bytes)?;
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|source| io_error(directory, source))?;
    Ok((digest, bytes))
}

fn freeze(file: &File, path: &Path) -> CrawlResult<()> {
    let mut permissions = file
        .metadata()
        .map_err(|source| io_error(path, source))?
        .permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)
        .and_then(|()| file.sync_all())
        .map_err(|source| io_error(path, source))
}

fn link(temporary: &Path, target: &Path) -> CrawlResult<()> {
    match std::fs::hard_link(temporary, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(source) => Err(io_error(target, source)),
    }
}

pub(super) fn verify(path: &Path, digest: &str, bytes: usize) -> CrawlResult<()> {
    super::limit("immutable request file bytes", bytes, MAX_BYTES)?;
    let metadata = std::fs::metadata(path).map_err(|source| io_error(path, source))?;
    let expected = u64::try_from(bytes)
        .map_err(|_| invariant("immutable request length exceeds file range"))?;
    if !metadata.is_file() || metadata.len() != expected || !metadata.permissions().readonly() {
        return Err(invariant(
            "immutable request file length or permissions differ from manifest",
        ));
    }
    let mut file = File::open(path).map_err(|source| io_error(path, source))?;
    let mut writer = HashWriter::default();
    io::copy(&mut file, &mut writer).map_err(|source| io_error(path, source))?;
    if format!("{:x}", writer.hash.finalize()) != digest {
        return Err(invariant(
            "immutable request file SHA256 differs from manifest",
        ));
    }
    Ok(())
}

fn encode(writer: &mut impl Write, value: &impl Serialize) -> CrawlResult<()> {
    serde_json::to_writer(writer, value).map_err(|source| CrawlError::Encode {
        table: "immutable result request input".into(),
        source,
    })
}

pub(super) fn io_error(path: &Path, source: io::Error) -> CrawlError {
    CrawlError::Io {
        path: PathBuf::from(path),
        source,
    }
}
fn exhausted() -> io::Error {
    io::Error::other("immutable request byte admission exhausted")
}
