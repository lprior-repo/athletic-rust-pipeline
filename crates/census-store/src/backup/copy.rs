
use std::fs::{self, File};
use std::io;
use std::path::Path;

use super::errors::{io_err, refused};
use super::generation::Generation;
use super::manifest::{digest_tree, table_row_counts, write_manifest};
use super::tree::{copy_tree, DB_DIR, LOCK_FILE};
use super::{BackupReport, Manifest, MANIFEST_PATH, MANIFEST_VERSION, STAGING_PREFIX};
use crate::{Store, StoreError, StoreResult};

impl Store {
    pub fn backup(from: &Path, to: &Path) -> StoreResult<BackupReport> {
        let start = std::time::Instant::now();
        let _closed = ClosedStore::acquire(from)?;
        check_backup_destination(from, to)?;
        let generation = Generation::create(to, STAGING_PREFIX)?;
        let copied = copy_tree(from, generation.path())?;
        let tables = count_published_generation(generation.path())?;
        let files = digest_tree(generation.path())?;
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            written_at: chrono::Utc::now().to_rfc3339(),
            files,
            tables: tables.clone(),
        };
        write_manifest(generation.path(), &manifest)?;
        generation.publish()?;
        Ok(BackupReport {
            to: to.display().to_string(),
            files: copied.files,
            bytes: copied.bytes,
            tables,
            elapsed_ms: start.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        })
    }
}

struct ClosedStore {
    _lock: File,
}

impl ClosedStore {
    fn acquire(root: &Path) -> StoreResult<Self> {
        let database = root.join(DB_DIR);
        if !database.is_dir() {
            return Err(refused(format!(
                "the store at {} has no {DB_DIR} directory: there is no database there to back up",
                root.display()
            )));
        }
        let lock = database.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock)
            .map_err(|error| match error.kind() {
                io::ErrorKind::NotFound => refused(format!(
                    "the database at {} has no lock file ({}) so fjall cannot open it either: nothing \
                     there is a store yet",
                    root.display(),
                    lock.display()
                )),
                _ => io_err(&lock, error),
            })?;
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => refused(format!(
                "the store at {} is open ({} is locked): a backup is a cold copy, so stop the writer \
                 first, then back up the stopped store",
                root.display(),
                lock.display()
            )),
            std::fs::TryLockError::Error(source) => StoreError::Io {
                path: lock.clone(),
                source,
            },
        })?;
        Ok(Self { _lock: file })
    }
}

fn count_published_generation(
    generation: &Path,
) -> StoreResult<std::collections::BTreeMap<String, u64>> {
    let store = Store::open(generation).map_err(|error| {
        refused(format!(
            "the backup copy at {} does not open as a store, so it was not published: {error}",
            generation.display()
        ))
    })?;
    let counts = table_row_counts(&store)?;
    drop(store);
    Ok(counts)
}

fn check_backup_destination(from: &Path, to: &Path) -> StoreResult<()> {
    if to.starts_with(from) {
        return Err(refused(format!(
            "destination {} is inside the store at {}: a backup cannot be written into the tree it is \
             copying",
            to.display(),
            from.display()
        )));
    }
    let kind = match fs::symlink_metadata(to) {
        Ok(meta) => meta.file_type(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io_err(to, source)),
    };
    if !kind.is_dir() {
        return Err(refused(format!(
            "destination {} is not a directory",
            to.display()
        )));
    }
    let mut entries = fs::read_dir(to).map_err(|source| io_err(to, source))?;
    match entries.next() {
        None => Ok(()),
        Some(_) if to.join(MANIFEST_PATH).is_file() => Ok(()),
        Some(_) => Err(refused(format!(
            "destination {} is not empty and is not a backup",
            to.display()
        ))),
    }
}
