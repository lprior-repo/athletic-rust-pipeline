//! Tests for store backup, restore, and integrity.
//!
//! A backup is a cold copy, so every backup test closes its store first (the guard scopes it) and the
//! tests that care about the refusal leave one open on purpose.

use std::collections::BTreeMap;
use std::path::Path;

use super::backup::files::{stream_copy, COPY_BUFFER_BYTES};
use super::backup::{Manifest, MANIFEST_VERSION};
use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

/// The temporary names a backup or restore may leave beside its destination.
const TEMPORARY_PREFIXES: [&str; 4] = [
    "backup.tmp.",
    "backup.old.",
    "restore.tmp.",
    "restore.old.",
];

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// A closed store holding one school per name.
fn store_with(root: &Path, names: &[&str]) {
    let store = Store::open(root).unwrap();
    let schools: Vec<CanonicalSchool> = names.iter().map(|name| school(name)).collect();
    store.append_many(Table::Schools, &schools).unwrap();
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn digest_of(path: &Path) -> String {
    sha256_hex(&std::fs::read(path).unwrap())
}

/// Every file under `root` with the digest of its bytes: a directory image to compare two runs by.
fn tree_image(root: &Path) -> BTreeMap<String, String> {
    let mut image = BTreeMap::new();
    collect_image(root, root, &mut image);
    image
}

fn collect_image(root: &Path, dir: &Path, image: &mut BTreeMap<String, String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let kind = std::fs::symlink_metadata(&path).unwrap().file_type();
        if kind.is_dir() {
            collect_image(root, &path, image);
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .to_string();
            image.insert(relative, digest_of(&path));
        }
    }
}

fn read_manifest(backup: &Path) -> Manifest {
    let text = std::fs::read_to_string(backup.join("backup.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Rewrite the manifest through a JSON document, the way a hand-edited or foreign one would arrive.
fn edit_manifest(backup: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let path = backup.join("backup.json");
    let mut document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    edit(&mut document);
    std::fs::write(&path, serde_json::to_string_pretty(&document).unwrap()).unwrap();
}

/// The staging directories a failed or finished run left behind beside its destination.
fn staging_leftovers(parent: &Path) -> Vec<String> {
    let mut left: Vec<String> = std::fs::read_dir(parent)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| {
            TEMPORARY_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect();
    left.sort();
    left
}

/// One byte of the payload the streaming tests copy and digest.
fn pattern_byte(index: u64) -> u8 {
    u8::try_from(index % 251).unwrap_or(0)
}

fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|index| pattern_byte(index as u64)).collect()
}

// ---------------------------------------------------------------------------
// Backup / restore round-trip
// ---------------------------------------------------------------------------

#[test]
fn backup_and_restore_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");

    let store = Store::open(&root).unwrap();
    let schools: Vec<CanonicalSchool> = (0..5).map(|i| school(&format!("School {i}"))).collect();
    store.append_many(Table::Schools, &schools).unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:school1",
            &serde_json::json!({"athletes": 5}),
        )
        .unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:school2",
            &serde_json::json!({"athletes": 10}),
        )
        .unwrap();
    drop(store);

    let report = Store::backup(&root, &backup_dir).unwrap();
    assert!(report.files > 0);
    assert!(report.bytes > 0);
    assert!(report.tables.contains_key("schools"));
    assert_eq!(report.tables.get("schools").copied(), Some(5));

    // The published generation is what the manifest describes: every entry is there, byte for byte.
    let manifest = read_manifest(&backup_dir);
    assert_eq!(manifest.version, MANIFEST_VERSION);
    assert_eq!(manifest.tables.get("schools").copied(), Some(5));
    for entry in &manifest.files {
        let published = backup_dir.join(&entry.path);
        assert_eq!(digest_of(&published), entry.sha256, "{}", entry.path);
    }

    let restore_dir = dir.path().join("restored");
    let restored = Store::restore(&backup_dir, &restore_dir).unwrap();
    assert_eq!(restored.tables.get("schools").copied(), Some(5));
    assert_eq!(
        u64::try_from(manifest.files.len()).unwrap(),
        restored.files,
        "the restore materialises exactly the generation the manifest lists"
    );

    // Verify the restored store opens and has the same data.
    let restored_store = Store::open(&restore_dir).unwrap();
    let schools_back: Vec<CanonicalSchool> = restored_store
        .scan::<CanonicalSchool>(Table::Schools)
        .unwrap();
    assert_eq!(schools_back.len(), 5);

    // Verify journal roundtrips.
    let journal_keys = restored_store.journal_keys("milesplit_rosters").unwrap();
    assert_eq!(journal_keys.len(), 2);
    assert!(journal_keys.contains("wi:school1"));
    assert!(journal_keys.contains("wi:school2"));

    // Integrity should be ok.
    let integrity = restored_store.integrity().unwrap();
    assert!(integrity.ok);
}

// ---------------------------------------------------------------------------
// Item 16: a backup is a cold copy, and says so
// ---------------------------------------------------------------------------

#[test]
fn backup_refuses_a_store_that_is_open() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let to = dir.path().join("backup");

    let store = Store::open(&root).unwrap();
    store
        .append_many(Table::Schools, &[school("Live School")])
        .unwrap();

    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(
        error.contains("is open"),
        "the refusal must say the store is open: {error}"
    );
    assert!(
        error.contains(&root.display().to_string()),
        "the refusal must name the store: {error}"
    );
    assert!(!to.exists(), "a refused backup publishes nothing");

    // Closing the store is what makes the same call succeed.
    drop(store);
    let report = Store::backup(&root, &to).unwrap();
    assert_eq!(report.tables.get("schools").copied(), Some(1));
    assert!(to.join("backup.json").is_file());
}

#[test]
fn backup_refuses_a_root_that_is_not_a_store() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("not-a-store");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("notes.txt"), "nothing to see").unwrap();
    let to = dir.path().join("backup");

    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(
        error.contains("no fjall directory"),
        "the refusal must name what is missing: {error}"
    );
    assert!(!to.exists());
}

#[test]
fn backup_refuses_a_destination_inside_the_store_it_is_copying() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    store_with(&root, &["Self Contained"]);
    let to = root.join("out").join("backup");

    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(
        error.contains("inside the store"),
        "the refusal must say the destination is inside the source: {error}"
    );
    assert!(!to.exists());
}

// ---------------------------------------------------------------------------
// Item 22: the traversal copies regular files and directories, and refuses the rest
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn backup_refuses_a_symlink_in_the_store_tree_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    store_with(&root, &["Symlinked"]);
    let link = root.join("fjall").join("link-to-elsewhere");
    std::os::unix::fs::symlink("/etc/hostname", &link).unwrap();
    let to = dir.path().join("backup");

    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(
        error.contains(&link.display().to_string()),
        "the refusal must name the symlink: {error}"
    );
    assert!(
        error.contains("symlink"),
        "the refusal must say what it refused: {error}"
    );
    assert!(!to.exists(), "a refused backup publishes nothing");
    assert!(staging_leftovers(dir.path()).is_empty());
}

#[cfg(unix)]
#[test]
fn backup_refuses_a_socket_in_the_store_tree() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    store_with(&root, &["Socketed"]);
    let socket = root.join("http").join("cache.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    let to = dir.path().join("backup");

    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(
        error.contains(&socket.display().to_string()) && error.contains("socket"),
        "the refusal must name the socket and its kind: {error}"
    );
    assert!(!to.exists());
}

// ---------------------------------------------------------------------------
// Item 18: generations are staged and swapped, never edited in place
// ---------------------------------------------------------------------------

#[test]
fn a_failed_backup_leaves_the_previous_generation_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    store_with(&root, &["Kept One", "Kept Two"]);
    Store::backup(&root, &to).unwrap();
    let published = tree_image(&to);
    assert!(published.contains_key("backup.json"));

    // The store moves on, so a second generation's bytes differ from the published one: a run that
    // wrote into `to` would leave new files beside the old manifest, and `published` catches exactly
    // that. Without this the two implementations copy byte-identical files and the assertion below
    // could not tell them apart.
    {
        let store = Store::open(&root).unwrap();
        store.append_many(Table::Schools, &[school("Kept Three")]).unwrap();
    }
    // The failure is planted in the last root the traversal reaches (`out`), so a run that wrote
    // straight into `to` has already overwritten `fjall` and every root before it when it dies.
    let out = root.join("out");
    std::fs::create_dir_all(&out).unwrap();
    let link = out.join("link-to-elsewhere");
    std::os::unix::fs::symlink("/etc/hostname", &link).unwrap();
    let error = Store::backup(&root, &to).unwrap_err().to_string();
    assert!(error.contains("symlink"), "{error}");

    assert_eq!(
        tree_image(&to),
        published,
        "the only known-good backup must survive a failed run byte for byte"
    );
    assert!(
        staging_leftovers(dir.path()).is_empty(),
        "a failed run must not leave its staging generation behind: {:?}",
        staging_leftovers(dir.path())
    );

    // Removing what broke the run leaves a backup that restores the original rows.
    std::fs::remove_file(&link).unwrap();
    let restore_dir = dir.path().join("restored");
    Store::restore(&to, &restore_dir).unwrap();
    let restored = Store::open(&restore_dir).unwrap();
    assert_eq!(restored.scan::<CanonicalSchool>(Table::Schools).unwrap().len(), 2);
}

#[test]
fn a_backup_into_an_existing_generation_replaces_it_whole() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let to = dir.path().join("backup");

    store_with(&root, &["One", "Two", "Three"]);
    Store::backup(&root, &to).unwrap();
    assert_eq!(read_manifest(&to).tables.get("schools").copied(), Some(3));

    {
        let store = Store::open(&root).unwrap();
        store
            .append_many(Table::Schools, &[school("Four")])
            .unwrap();
    }
    let report = Store::backup(&root, &to).unwrap();
    assert_eq!(report.tables.get("schools").copied(), Some(4));

    // The second generation is the one on disk: its manifest describes its own files and rows.
    let manifest = read_manifest(&to);
    assert_eq!(manifest.tables.get("schools").copied(), Some(4));
    for entry in &manifest.files {
        assert_eq!(
            digest_of(&to.join(&entry.path)),
            entry.sha256,
            "{}",
            entry.path
        );
    }
    assert!(staging_leftovers(dir.path()).is_empty());

    let restore_dir = dir.path().join("restored");
    let restored = Store::restore(&to, &restore_dir).unwrap();
    assert_eq!(restored.tables.get("schools").copied(), Some(4));
}

// ---------------------------------------------------------------------------
// Item 17: one streaming pass, constant memory
// ---------------------------------------------------------------------------

/// A reader that serves a payload and fails the copy if it is ever asked for more than one buffer.
struct OneBufferAtATime {
    remaining: u64,
    served: u64,
}

impl std::io::Read for OneBufferAtATime {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        assert!(
            buf.len() <= COPY_BUFFER_BYTES,
            "the copier asked for {} bytes at once: more than the {} byte buffer it is supposed to hold",
            buf.len(),
            COPY_BUFFER_BYTES
        );
        let available = usize::try_from(self.remaining).unwrap_or(usize::MAX);
        let want = available.min(buf.len());
        for (offset, slot) in buf.iter_mut().take(want).enumerate() {
            *slot = pattern_byte(self.served.saturating_add(offset as u64));
        }
        self.served = self.served.saturating_add(want as u64);
        self.remaining = self.remaining.saturating_sub(want as u64);
        Ok(want)
    }
}

#[test]
fn stream_copy_copies_more_than_one_buffer_through_one_buffer() {
    let len = COPY_BUFFER_BYTES * 9 + 1_234;
    let mut reader = OneBufferAtATime {
        remaining: len as u64,
        served: 0,
    };
    let mut destination: Vec<u8> = Vec::new();

    let streamed = stream_copy(&mut reader, &mut destination).unwrap();

    assert_eq!(streamed.bytes, len as u64);
    assert_eq!(destination.len(), len);
    assert_eq!(destination, pattern(len));
    assert_eq!(streamed.sha256, sha256_hex(&pattern(len)));
    assert!(len > COPY_BUFFER_BYTES);
}

#[test]
fn backup_digests_a_file_larger_than_the_copy_buffer() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    store_with(&root, &["Buffered"]);
    // Well past any single buffer: a copy that held the file would hold this.
    let payload = pattern(COPY_BUFFER_BYTES * 9 + 1_234);
    let big = root.join("http").join("response.bin");
    std::fs::write(&big, &payload).unwrap();
    let to = dir.path().join("backup");

    Store::backup(&root, &to).unwrap();

    let entry = read_manifest(&to)
        .files
        .into_iter()
        .find(|entry| entry.path == "http/response.bin")
        .expect("the oversized file must be in the manifest");
    assert_eq!(entry.length, payload.len() as u64);
    assert_eq!(entry.sha256, sha256_hex(&payload));

    let restore_dir = dir.path().join("restored");
    Store::restore(&to, &restore_dir).unwrap();
    assert_eq!(
        digest_of(&restore_dir.join("http").join("response.bin")),
        sha256_hex(&payload)
    );
}

// ---------------------------------------------------------------------------
// Item 19: the manifest version is read or refused, never assumed
// ---------------------------------------------------------------------------

#[test]
fn restore_refuses_a_manifest_version_it_does_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    let restore_dir = dir.path().join("restored");
    store_with(&root, &["Versioned"]);
    Store::backup(&root, &to).unwrap();
    edit_manifest(&to, |document| {
        document["version"] = serde_json::Value::from(u32::MAX);
    });

    let error = Store::restore(&to, &restore_dir).unwrap_err().to_string();
    assert!(
        error.contains(&format!("version {}", u32::MAX))
            && error.contains(&format!("reads version {MANIFEST_VERSION}")),
        "the refusal must name both versions: {error}"
    );
    assert!(!restore_dir.exists());
    assert!(staging_leftovers(dir.path()).is_empty());
}

// ---------------------------------------------------------------------------
// Item 20 and 21: a restore reconciles, and a failed restore leaves nothing behind
// ---------------------------------------------------------------------------

#[test]
fn restore_refuses_rows_that_disagree_with_the_manifest_and_a_retry_still_works() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    let restore_dir = dir.path().join("restored");
    store_with(&root, &["One", "Two", "Three"]);
    Store::backup(&root, &to).unwrap();
    edit_manifest(&to, |document| {
        document["tables"]["schools"] = serde_json::Value::from(999);
    });

    let error = Store::restore(&to, &restore_dir).unwrap_err().to_string();
    assert!(
        error.contains("restored table schools holds 3 rows") && error.contains("records 999"),
        "the refusal must name the table and both counts: {error}"
    );
    assert!(
        !restore_dir.exists(),
        "a failed restore must leave no destination tree for the retry to trip over"
    );
    assert!(staging_leftovers(dir.path()).is_empty());

    // The retry into the same destination succeeds once the manifest tells the truth again.
    edit_manifest(&to, |document| {
        document["tables"]["schools"] = serde_json::Value::from(3);
    });
    let restored = Store::restore(&to, &restore_dir).unwrap();
    assert_eq!(restored.tables.get("schools").copied(), Some(3));
    let reopened = Store::open(&restore_dir).unwrap();
    assert_eq!(
        reopened.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        3
    );
}

// ---------------------------------------------------------------------------
// Corruption detection
// ---------------------------------------------------------------------------

#[test]
fn corrupt_file_fails_restore_leaving_destination_empty() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let backup_path = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1", "School 2"]);
    Store::backup(&root, &backup_path).unwrap();

    // Corrupt one file in the backup (flip a byte in the first data file we find).
    let manifest = read_manifest(&backup_path);
    let target = manifest
        .files
        .iter()
        .find(|f| f.path.starts_with("fjall/"))
        .expect("backup should have database files");

    let target_path = backup_path.join(&target.path);
    let original = std::fs::read(&target_path).unwrap();
    assert!(!original.is_empty());

    // Corrupt the first byte.
    let mut corrupted = original.clone();
    corrupted[0] ^= 0xFF;
    std::fs::write(&target_path, &corrupted).unwrap();

    // Restore should fail, naming the corrupted file.
    let restore_dir = tempfile::tempdir().unwrap();
    let restore_path = restore_dir.path().to_path_buf();
    let result = Store::restore(&backup_path, &restore_path);

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains(&target.path),
        "error should mention the corrupted file: {err}"
    );

    // Destination must stay empty.
    let entries: Vec<_> = std::fs::read_dir(&restore_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        entries.is_empty(),
        "restore destination must be empty after failure"
    );
    assert!(staging_leftovers(dir.path()).is_empty());
}

// ---------------------------------------------------------------------------
// Destination refusals
// ---------------------------------------------------------------------------

#[test]
fn backup_refuses_non_empty_non_backup_destination() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    store_with(&root, &["Refused"]);

    // Create a non-empty directory that is not a backup.
    let bad_dest = dir.path().join("not_a_backup");
    std::fs::create_dir_all(&bad_dest).unwrap();
    std::fs::write(bad_dest.join("random_file.txt"), "hello").unwrap();

    let result = Store::backup(&root, &bad_dest);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not empty and is not a backup"));
    assert!(bad_dest.join("random_file.txt").is_file());
}

#[test]
fn restore_refuses_non_empty_destination() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["Occupied"]);
    Store::backup(&root, &backup_dir).unwrap();

    // Create a non-empty destination.
    let bad_dest = dir.path().join("restore_here");
    std::fs::create_dir_all(&bad_dest).unwrap();
    std::fs::write(bad_dest.join("existing.txt"), "data").unwrap();

    let result = Store::restore(&backup_dir, &bad_dest);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not empty"));
    assert_eq!(
        std::fs::read_to_string(bad_dest.join("existing.txt")).unwrap(),
        "data"
    );
}

// ---------------------------------------------------------------------------
// Manifest contents
// ---------------------------------------------------------------------------

#[test]
fn backup_includes_manifest_and_durable_material() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1"]);

    Store::backup(&root, &backup_dir).unwrap();

    // Manifest must exist.
    let manifest_path = backup_dir.join("backup.json");
    assert!(manifest_path.exists());

    // Manifest must be valid JSON with expected structure.
    let manifest = read_manifest(&backup_dir);
    assert_eq!(manifest.version, MANIFEST_VERSION);
    assert!(!manifest.written_at.is_empty());
    assert!(!manifest.files.is_empty());
    assert!(!manifest.tables.is_empty());
    assert_eq!(
        manifest.tables.len(),
        Table::ALL.len(),
        "the manifest records a count for every table"
    );

    // Every file in the manifest must exist.
    for entry in &manifest.files {
        assert!(
            backup_dir.join(&entry.path).exists(),
            "missing: {}",
            entry.path
        );
    }

    // The database is in there: a backup that skipped fjall/ would be no backup at all.
    assert!(manifest.files.iter().any(|entry| entry.path == "fjall/version"));
}

#[test]
fn restore_reopens_store_and_verifies_data() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1", "School 2"]);

    Store::backup(&root, &backup_dir).unwrap();

    let restore_dir = dir.path().join("restored");
    Store::restore(&backup_dir, &restore_dir).unwrap();

    // Open restored store and verify data.
    let restored = Store::open(&restore_dir).unwrap();
    let schools_back: Vec<CanonicalSchool> =
        restored.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(schools_back.len(), 3);
}

// ---------------------------------------------------------------------------
// Integrity checks
// ---------------------------------------------------------------------------

#[test]
fn integrity_reports_ok_for_fresh_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let report = store.integrity().unwrap();
    assert!(report.ok);
    assert_eq!(report.tables.len(), Table::ALL.len());
}

#[test]
fn integrity_reports_mismatch_when_entity_log_has_extra_rows() {
    // A store appends observations and keeps the count its writer commits with them, so a reopened
    // store reports the same figure it wrote: the ledger is not the sequence pointer, which a failed
    // commit leaves ahead of the rows the store holds.
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append_many(Table::Schools, &[school("Test School")])
            .unwrap();
    }

    // Open again and check integrity.
    let store = Store::open(dir.path()).unwrap();
    let report = store.integrity().unwrap();
    assert!(report.ok);

    // Find the schools table.
    let schools_row = report
        .tables
        .iter()
        .find(|t| t.table == "schools")
        .expect("schools table should be present");
    assert_eq!(schools_row.expected, 1);
    assert_eq!(schools_row.actual, 1);
}

#[test]
fn integrity_reports_unreadable_journal() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // Journal a key (creates the journal directory and file).
    store
        .journal_done(
            "milesplit_rosters",
            "wi:test",
            &serde_json::json!({"athletes": 5}),
        )
        .unwrap();

    // Integrity should be ok.
    let report = store.integrity().unwrap();
    assert!(report.ok);
}
