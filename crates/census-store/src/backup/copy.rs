//! Store backup implementation.
//!
//! A backup is a **cold** copy: the database must be closed, and this module proves it is before it
//! copies anything. See the module docs of [`super`] for why fjall 3.1.10 offers no alternative.

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
    /// Copy the closed store at `from` into `to` and write `backup.json` beside it.
    ///
    /// `from` is a store *root* - the directory [`Store::open`] was given - and it must not be open:
    /// this refuses a store whose database lock is held, because a recursive copy of an open LSM tree
    /// is not a snapshot and this function will not label one as a backup. Stop the writer (the
    /// endpoint serving the root), then back up the stopped store; `tools/ops-backup-drill.sh` does the
    /// same thing by hand.
    ///
    /// The generation is built in a staging directory beside `to` and renamed onto it once every file
    /// and the manifest are written, fsynced and digested. `to` may not exist, may be an empty
    /// directory, or may hold an earlier backup - which is replaced whole, never edited in place - and
    /// any other destination is refused. A run that fails leaves `to` exactly as it was.
    ///
    /// The finished copy is opened once before it is published: a generation that is not a store is not
    /// a backup, and that open is also where each table's row count is taken for the manifest. The copy
    /// is a faithful *store*, not necessarily a byte image of the source - the open may settle or
    /// migrate the copy (fjall recovery, a legacy import whose marker travels with it) - and the
    /// manifest's digests describe the bytes that were published.
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

/// The database's advisory lock, held for as long as the store must stay closed.
///
/// fjall takes `fjall/lock` with `std::fs::File::try_lock` when it opens a database
/// (`fjall-3.1.10/src/locked_file.rs:49-80`, taken at `src/db.rs:569`, `src/db.rs:815`), so taking the
/// same lock on the same file is exactly the question "is this database open?" - for another process
/// and for this one, since `flock` locks an open file description and a second open of the same file
/// is a second description. A held lock refuses the backup; a lock that is free is held until this
/// guard drops.
struct ClosedStore {
    /// Dropping the file releases the lock.
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

/// Open the finished generation as a store and count what its keyspaces hold.
///
/// Two things happen here, in this order, and the order matters: the copy is proven to be a store
/// rather than a directory of files, and its per-table row counts are taken from the keyspace itself.
/// It runs before the digests are taken, so the manifest describes the bytes that were published even
/// when this open wrote to the copy.
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

/// Refuse a destination a generation cannot be published onto.
///
/// An absent directory is created by the publish rename; an empty one or an earlier backup is replaced
/// whole; anything else is refused rather than written into.
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
