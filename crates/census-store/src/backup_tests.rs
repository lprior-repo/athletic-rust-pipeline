use std::collections::BTreeMap;
use std::path::Path;

use super::backup::files::{
    stream_copy, sync_directories, sync_directories_with, COPY_BUFFER_BYTES,
};
use super::backup::{Manifest, MANIFEST_VERSION};
use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const TEMPORARY_PREFIXES: [&str; 4] =
    ["backup.tmp.", "backup.old.", "restore.tmp.", "restore.old."];

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn store_with(root: &Path, names: &[&str]) -> TestResult {
    let store = Store::open(root)?;
    let schools: Vec<CanonicalSchool> = names.iter().map(|name| school(name)).collect();
    store.append_many(Table::Schools, &schools)?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

fn digest_of(path: &Path) -> TestResult<String> {
    Ok(sha256_hex(&std::fs::read(path)?))
}

fn tree_image(root: &Path) -> TestResult<BTreeMap<String, String>> {
    let mut image = BTreeMap::new();
    collect_image(root, root, &mut image)?;
    Ok(image)
}

fn collect_image(root: &Path, dir: &Path, image: &mut BTreeMap<String, String>) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let kind = std::fs::symlink_metadata(&path)?.file_type();
        if kind.is_dir() {
            collect_image(root, &path, image)?;
        } else if kind.is_file() {
            let relative = path.strip_prefix(root)?.to_string_lossy().to_string();
            image.insert(relative, digest_of(&path)?);
        }
    }
    Ok(())
}

fn read_manifest(backup: &Path) -> TestResult<Manifest> {
    let text = std::fs::read_to_string(backup.join("backup.json"))?;
    Ok(serde_json::from_str(&text)?)
}

fn edit_manifest(backup: &Path, edit: impl FnOnce(&mut serde_json::Value)) -> TestResult {
    let path = backup.join("backup.json");
    let mut document: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    edit(&mut document);
    std::fs::write(&path, serde_json::to_string_pretty(&document)?)?;
    Ok(())
}

fn staging_leftovers(parent: &Path) -> TestResult<Vec<String>> {
    let mut left = Vec::new();
    for entry in std::fs::read_dir(parent)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if TEMPORARY_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
        {
            left.push(name);
        }
    }
    left.sort();
    Ok(left)
}

fn pattern_byte(index: u64) -> u8 {
    u8::try_from(index % 251).map_or(0, |value| value)
}
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|index| pattern_byte(index as u64)).collect()
}

#[test]
fn backup_and_restore_roundtrip() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    let store = Store::open(&root)?;
    let schools: Vec<CanonicalSchool> = (0..5).map(|i| school(&format!("School {i}"))).collect();
    store.append_many(Table::Schools, &schools)?;
    store.journal_done(
        "milesplit_rosters",
        "wi:school1",
        &serde_json::json!({"athletes": 5}),
    )?;
    store.journal_done(
        "milesplit_rosters",
        "wi:school2",
        &serde_json::json!({"athletes": 10}),
    )?;
    drop(store);
    let report = Store::backup(&root, &backup_dir)?;
    check!(report.files > 0);
    check!(report.bytes > 0);
    check!(report.tables.contains_key("schools"));
    check!(eq; report.tables.get("schools").copied(), Some(5));
    let manifest = read_manifest(&backup_dir)?;
    check!(eq; manifest.version, MANIFEST_VERSION);
    check!(eq; manifest.tables.get("schools").copied(), Some(5));
    for entry in &manifest.files {
        check!(eq; digest_of(&backup_dir.join(&entry.path))?, entry.sha256, "{}", entry.path);
    }
    let restore_dir = dir.path().join("restored");
    let restored = Store::restore(&backup_dir, &restore_dir)?;
    check!(eq; restored.tables.get("schools").copied(), Some(5));
    check!(eq; u64::try_from(manifest.files.len())?, restored.files, "the restore materialises exactly the generation the manifest lists");
    let restored_store = Store::open(&restore_dir)?;
    let schools_back: Vec<CanonicalSchool> =
        restored_store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; schools_back.len(), 5);
    let journal_keys = restored_store.journal_keys("milesplit_rosters")?;
    check!(eq; journal_keys.len(), 2);
    check!(journal_keys.contains("wi:school1"));
    check!(journal_keys.contains("wi:school2"));
    check!(restored_store.integrity()?.ok);
    Ok(())
}

#[test]
fn backup_refuses_a_store_that_is_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    let store = Store::open(&root)?;
    store.append_many(Table::Schools, &[school("Live School")])?;
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected an open-store refusal, got {report:?}").into()),
    };
    check!(
        error.contains("is open"),
        "the refusal must say the store is open: {error}"
    );
    check!(
        error.contains(&root.display().to_string()),
        "the refusal must name the store: {error}"
    );
    check!(!to.exists(), "a refused backup publishes nothing");
    drop(store);
    let report = Store::backup(&root, &to)?;
    check!(eq; report.tables.get("schools").copied(), Some(1));
    check!(to.join("backup.json").is_file());
    Ok(())
}

#[test]
fn backup_refuses_a_root_that_is_not_a_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("not-a-store");
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("notes.txt"), "nothing to see")?;
    let to = dir.path().join("backup");
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected a missing-store refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains("no fjall directory"),
        "the refusal must name what is missing: {error}"
    );
    check!(!to.exists());
    Ok(())
}

#[test]
fn backup_refuses_a_destination_inside_the_store_it_is_copying() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Self Contained"])?;
    let to = root.join("out").join("backup");
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected an inside-source refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains("inside the store"),
        "the refusal must say the destination is inside the source: {error}"
    );
    check!(!to.exists());
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_refuses_a_destination_aliased_into_the_store_tree() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Aliased"])?;
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink(&root, &alias)?;
    let to = alias.join("http").join("backup");
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected an aliased-destination refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains("inside the store"),
        "the refusal must say the destination is inside the source: {error}"
    );
    check!(
        error.contains(&alias.display().to_string()),
        "the refusal must name the destination as given: {error}"
    );
    check!(!root.join("http").join("backup").exists());
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_refuses_an_inside_destination_when_the_store_path_is_an_alias() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Alias Source"])?;
    let alias = dir.path().join("alias");
    std::os::unix::fs::symlink(&root, &alias)?;
    let to = root.join("http").join("backup");
    let error = match Store::backup(&alias, &to) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected an aliased-source refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains("inside the store"),
        "the refusal must say the destination is inside the source: {error}"
    );
    check!(!to.exists(), "a refused backup publishes nothing");
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn backup_refuses_a_destination_above_the_store_it_is_copying() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Above"])?;
    let error = match Store::backup(&root, dir.path()) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected an above-source refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains("above the store"),
        "the refusal must say the destination would replace the source: {error}"
    );
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_refuses_a_symlink_in_the_store_tree_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Symlinked"])?;
    let link = root.join("fjall").join("link-to-elsewhere");
    std::os::unix::fs::symlink("/etc/hostname", &link)?;
    let to = dir.path().join("backup");
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected a symlink refusal, got {report:?}").into()),
    };
    check!(
        error.contains(&link.display().to_string()),
        "the refusal must name the symlink: {error}"
    );
    check!(
        error.contains("symlink"),
        "the refusal must say what it refused: {error}"
    );
    check!(!to.exists(), "a refused backup publishes nothing");
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_refuses_a_socket_in_the_store_tree() -> TestResult {
    let dir = tempfile::tempdir_in("/tmp")?;
    let root = dir.path().join("store");
    store_with(&root, &["Socketed"])?;
    let socket = root.join("http").join("cache.sock");
    let _listener = std::os::unix::net::UnixListener::bind(&socket)?;
    let to = dir.path().join("backup");
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected a socket refusal, got {report:?}").into()),
    };
    check!(
        error.contains(&socket.display().to_string()) && error.contains("socket"),
        "the refusal must name the socket and its kind: {error}"
    );
    check!(!to.exists());
    Ok(())
}

#[test]
fn a_failed_backup_leaves_the_previous_generation_untouched() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    store_with(&root, &["Kept One", "Kept Two"])?;
    Store::backup(&root, &to)?;
    let published = tree_image(&to)?;
    check!(published.contains_key("backup.json"));
    {
        let store = Store::open(&root)?;
        store.append_many(Table::Schools, &[school("Kept Three")])?;
    }
    let out = root.join("out");
    std::fs::create_dir_all(&out)?;
    let link = out.join("link-to-elsewhere");
    std::os::unix::fs::symlink("/etc/hostname", &link)?;
    let error = match Store::backup(&root, &to) {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected a symlink refusal, got {report:?}").into()),
    };
    check!(error.contains("symlink"), "{error}");
    check!(eq; tree_image(&to)?, published, "the only known-good backup must survive a failed run byte for byte");
    let leftovers = staging_leftovers(dir.path())?;
    check!(
        leftovers.is_empty(),
        "a failed run must not leave its staging generation behind: {leftovers:?}"
    );
    std::fs::remove_file(&link)?;
    let restore_dir = dir.path().join("restored");
    Store::restore(&to, &restore_dir)?;
    let restored = Store::open(&restore_dir)?;
    check!(eq; restored.scan::<CanonicalSchool>(Table::Schools)?.len(), 2);
    Ok(())
}

#[test]
fn a_backup_into_an_existing_generation_replaces_it_whole() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    store_with(&root, &["One", "Two", "Three"])?;
    Store::backup(&root, &to)?;
    check!(eq; read_manifest(&to)?.tables.get("schools").copied(), Some(3));
    {
        let store = Store::open(&root)?;
        store.append_many(Table::Schools, &[school("Four")])?;
    }
    let report = Store::backup(&root, &to)?;
    check!(eq; report.tables.get("schools").copied(), Some(4));
    let manifest = read_manifest(&to)?;
    check!(eq; manifest.tables.get("schools").copied(), Some(4));
    for entry in &manifest.files {
        check!(eq; digest_of(&to.join(&entry.path))?, entry.sha256, "{}", entry.path);
    }
    check!(staging_leftovers(dir.path())?.is_empty());
    let restore_dir = dir.path().join("restored");
    let restored = Store::restore(&to, &restore_dir)?;
    check!(eq; restored.tables.get("schools").copied(), Some(4));
    Ok(())
}

struct OneBufferAtATime {
    remaining: u64,
    served: u64,
}

impl std::io::Read for OneBufferAtATime {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() > COPY_BUFFER_BYTES {
            return Err(std::io::Error::other(format!("the copier asked for {} bytes at once: more than the {} byte buffer it is supposed to hold", buf.len(), COPY_BUFFER_BYTES)));
        }
        let available = usize::try_from(self.remaining).map_or(usize::MAX, |value| value);
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
fn stream_copy_copies_more_than_one_buffer_through_one_buffer() -> TestResult {
    let len = COPY_BUFFER_BYTES * 9 + 1_234;
    let mut reader = OneBufferAtATime {
        remaining: len as u64,
        served: 0,
    };
    let mut destination: Vec<u8> = Vec::new();
    let streamed = stream_copy(&mut reader, &mut destination)?;
    check!(eq; streamed.bytes, len as u64);
    check!(eq; destination.len(), len);
    check!(eq; destination, pattern(len));
    check!(eq; streamed.sha256, sha256_hex(&pattern(len)));
    check!(len > COPY_BUFFER_BYTES);
    Ok(())
}

#[test]
fn backup_digests_a_file_larger_than_the_copy_buffer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Buffered"])?;
    let payload = pattern(COPY_BUFFER_BYTES * 9 + 1_234);
    let big = root.join("http").join("response.bin");
    std::fs::write(&big, &payload)?;
    let to = dir.path().join("backup");
    Store::backup(&root, &to)?;
    let entry = read_manifest(&to)?
        .files
        .into_iter()
        .find(|entry| entry.path == "http/response.bin")
        .ok_or("the oversized file must be in the manifest")?;
    check!(eq; entry.length, payload.len() as u64);
    check!(eq; entry.sha256, sha256_hex(&payload));
    let restore_dir = dir.path().join("restored");
    Store::restore(&to, &restore_dir)?;
    check!(eq; digest_of(&restore_dir.join("http").join("response.bin"))?, sha256_hex(&payload));
    Ok(())
}

#[test]
fn restore_refuses_a_manifest_version_it_does_not_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    let restore_dir = dir.path().join("restored");
    store_with(&root, &["Versioned"])?;
    Store::backup(&root, &to)?;
    edit_manifest(&to, |document| {
        document["version"] = serde_json::Value::from(u32::MAX);
    })?;
    let error = match Store::restore(&to, &restore_dir) {
        Err(error) => error.to_string(),
        Ok(report) => {
            return Err(format!("expected a manifest-version refusal, got {report:?}").into())
        }
    };
    check!(
        error.contains(&format!("version {}", u32::MAX))
            && error.contains(&format!("reads version {MANIFEST_VERSION}")),
        "the refusal must name both versions: {error}"
    );
    check!(!restore_dir.exists());
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn restore_refuses_rows_that_disagree_with_the_manifest_and_a_retry_still_works() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let to = dir.path().join("backup");
    let restore_dir = dir.path().join("restored");
    store_with(&root, &["One", "Two", "Three"])?;
    Store::backup(&root, &to)?;
    edit_manifest(&to, |document| {
        document["tables"]["schools"] = serde_json::Value::from(999);
    })?;
    let error = match Store::restore(&to, &restore_dir) {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected a row-count refusal, got {report:?}").into()),
    };
    check!(
        error.contains("restored table schools holds 3 rows") && error.contains("records 999"),
        "the refusal must name the table and both counts: {error}"
    );
    check!(
        !restore_dir.exists(),
        "a failed restore must leave no destination tree for the retry to trip over"
    );
    check!(staging_leftovers(dir.path())?.is_empty());
    edit_manifest(&to, |document| {
        document["tables"]["schools"] = serde_json::Value::from(3);
    })?;
    let restored = Store::restore(&to, &restore_dir)?;
    check!(eq; restored.tables.get("schools").copied(), Some(3));
    let reopened = Store::open(&restore_dir)?;
    check!(eq; reopened.scan::<CanonicalSchool>(Table::Schools)?.len(), 3);
    Ok(())
}

#[test]
fn corrupt_file_fails_restore_leaving_destination_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let backup_path = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1", "School 2"])?;
    Store::backup(&root, &backup_path)?;
    let manifest = read_manifest(&backup_path)?;
    let target = manifest
        .files
        .iter()
        .find(|f| f.path.starts_with("fjall/"))
        .ok_or("backup should have database files")?;
    let target_path = backup_path.join(&target.path);
    let original = std::fs::read(&target_path)?;
    check!(!original.is_empty());
    let mut corrupted = original.clone();
    corrupted[0] ^= 0xFF;
    std::fs::write(&target_path, &corrupted)?;
    let restore_dir = tempfile::tempdir()?;
    let restore_path = restore_dir.path().to_path_buf();
    let result = Store::restore(&backup_path, &restore_path);
    check!(result.is_err());
    let err = match result {
        Err(error) => error.to_string(),
        Ok(report) => return Err(format!("expected a corrupt-file refusal, got {report:?}").into()),
    };
    check!(
        err.contains(&target.path),
        "error should mention the corrupted file: {err}"
    );
    let entries: Vec<_> = std::fs::read_dir(&restore_path)?.collect::<Result<_, _>>()?;
    check!(
        entries.is_empty(),
        "restore destination must be empty after failure"
    );
    check!(staging_leftovers(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn backup_refuses_non_empty_non_backup_destination() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with(&root, &["Refused"])?;
    let bad_dest = dir.path().join("not_a_backup");
    std::fs::create_dir_all(&bad_dest)?;
    std::fs::write(bad_dest.join("random_file.txt"), "hello")?;
    let result = Store::backup(&root, &bad_dest);
    check!(result.is_err());
    let error = match result {
        Err(error) => error,
        Ok(report) => {
            return Err(format!("expected an occupied-backup refusal, got {report:?}").into())
        }
    };
    check!(error.to_string().contains("not empty and is not a backup"));
    check!(bad_dest.join("random_file.txt").is_file());
    Ok(())
}

#[test]
fn restore_refuses_non_empty_destination() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["Occupied"])?;
    Store::backup(&root, &backup_dir)?;
    let bad_dest = dir.path().join("restore_here");
    std::fs::create_dir_all(&bad_dest)?;
    std::fs::write(bad_dest.join("existing.txt"), "data")?;
    let result = Store::restore(&backup_dir, &bad_dest);
    check!(result.is_err());
    let error = match result {
        Err(error) => error,
        Ok(report) => {
            return Err(format!("expected an occupied-restore refusal, got {report:?}").into())
        }
    };
    check!(error.to_string().contains("not empty"));
    check!(eq; std::fs::read_to_string(bad_dest.join("existing.txt"))?, "data");
    Ok(())
}

#[test]
fn backup_includes_manifest_and_durable_material() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1"])?;
    Store::backup(&root, &backup_dir)?;
    check!(backup_dir.join("backup.json").exists());
    let manifest = read_manifest(&backup_dir)?;
    check!(eq; manifest.version, MANIFEST_VERSION);
    check!(!manifest.written_at.is_empty());
    check!(!manifest.files.is_empty());
    check!(!manifest.tables.is_empty());
    check!(eq; manifest.tables.len(), Table::ALL.len(), "the manifest records a count for every table");
    for entry in &manifest.files {
        check!(
            backup_dir.join(&entry.path).exists(),
            "missing: {}",
            entry.path
        );
    }
    check!(manifest
        .files
        .iter()
        .any(|entry| entry.path == "fjall/version"));
    Ok(())
}

#[test]
fn restore_reopens_store_and_verifies_data() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let backup_dir = dir.path().join("backup");
    store_with(&root, &["School 0", "School 1", "School 2"])?;
    Store::backup(&root, &backup_dir)?;
    let restore_dir = dir.path().join("restored");
    Store::restore(&backup_dir, &restore_dir)?;
    let restored = Store::open(&restore_dir)?;
    let schools_back: Vec<CanonicalSchool> = restored.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; schools_back.len(), 3);
    Ok(())
}

#[test]
fn integrity_reports_ok_for_fresh_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let report = store.integrity()?;
    check!(report.ok);
    check!(eq; report.tables.len(), Table::ALL.len());
    Ok(())
}

#[test]
fn integrity_reports_mismatch_when_entity_log_has_extra_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        store.append_many(Table::Schools, &[school("Test School")])?;
    }
    let store = Store::open(dir.path())?;
    let report = store.integrity()?;
    check!(report.ok);
    let schools_row = report
        .tables
        .iter()
        .find(|t| t.table == "schools")
        .ok_or("schools table should be present")?;
    check!(eq; schools_row.expected, 1);
    check!(eq; schools_row.actual, 1);
    Ok(())
}

#[test]
fn integrity_reports_unreadable_journal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(
        "milesplit_rosters",
        "wi:test",
        &serde_json::json!({"athletes": 5}),
    )?;
    check!(store.integrity()?.ok);
    Ok(())
}

fn refusal_of(result: StoreResult<RestoreReport>) -> TestResult<String> {
    match result {
        Err(error) => Ok(error.to_string()),
        Ok(report) => Err(format!("expected a refusal, got {report:?}").into()),
    }
}

#[test]
fn sync_directories_reaches_every_directory_bottom_up() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("tree");
    std::fs::create_dir_all(root.join("a/b/c"))?;
    std::fs::create_dir_all(root.join("d"))?;
    std::fs::write(root.join("a/b/c/evidence"), b"retained")?;
    let mut seen: Vec<std::path::PathBuf> = Vec::new();
    let count = sync_directories_with(&root, &mut |path| {
        seen.push(path.to_path_buf());
        Ok(())
    })?;
    check!(eq; count, 5, "the root, a, a/b, a/b/c and d are every directory");
    check!(eq; seen.len(), 5, "each directory syncs exactly once");
    let root_position = seen
        .iter()
        .position(|path| path == &root)
        .ok_or("the walk must sync the root")?;
    check!(
        eq;
        root_position,
        seen.len() - 1,
        "the root syncs after the directories it holds"
    );
    for (position, path) in seen.iter().enumerate() {
        if position == root_position {
            continue;
        }
        let parent = path.parent().ok_or("a nested directory has no parent")?;
        let parent_position = seen
            .iter()
            .position(|candidate| candidate == parent)
            .ok_or_else(|| format!("{} was never synced", parent.display()))?;
        check!(
            parent_position > position,
            "{} must sync after {}",
            parent.display(),
            path.display()
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn sync_directories_reports_an_unreadable_nested_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("tree");
    let locked = root.join("locked");
    std::fs::create_dir_all(locked.join("inner"))?;
    let mut permissions = std::fs::metadata(&locked)?.permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&locked, permissions)?;
    let result = sync_directories(&root);
    let mut permissions = std::fs::metadata(&locked)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&locked, permissions)?;
    let error = match result {
        Err(error) => error.to_string(),
        Ok(count) => {
            return Err(format!("expected a refusal, got {count} synced directories").into())
        }
    };
    check!(
        error.contains(&locked.display().to_string()),
        "the failure must name the directory it could not read: {error}"
    );
    Ok(())
}

fn edit_manifest_entry(
    backup: &Path,
    path: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) -> TestResult {
    let mut edited = false;
    let mut edit = Some(edit);
    edit_manifest(backup, |document| {
        let entry = match document["files"]
            .as_array_mut()
            .and_then(|entries| entries.iter_mut().find(|entry| entry["path"] == path))
        {
            Some(entry) => entry,
            None => return,
        };
        if let Some(edit) = edit.take() {
            edit(entry);
            edited = true;
        }
    })?;
    if !edited {
        return Err(format!("the backup manifest records no entry for {path}").into());
    }
    Ok(())
}

#[cfg(unix)]
fn store_with_publication(root: &Path) -> TestResult {
    store_with(root, &["Published"])?;
    let generation = root.join("out/publication/generations/g1");
    std::fs::create_dir_all(&generation)?;
    std::fs::write(generation.join("census.xlsx"), b"workbook")?;
    std::os::unix::fs::symlink("generations/g1", root.join("out/publication/current"))?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_and_restore_carry_a_store_relative_publication_pointer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with_publication(&root)?;
    let to = dir.path().join("backup");
    let report = Store::backup(&root, &to)?;
    check!(eq; report.links, 1, "the pointer must be copied as a link");
    check!(eq;
        std::fs::read_link(to.join("out/publication/current"))?,
        std::path::PathBuf::from("generations/g1"),
        "the backup must hold the pointer itself, not what it names"
    );
    let restored = dir.path().join("restored");
    let report = Store::restore(&to, &restored)?;
    check!(eq; report.links, 1, "restore must recreate the pointer");
    check!(eq;
        std::fs::read_link(restored.join("out/publication/current"))?,
        std::path::PathBuf::from("generations/g1"),
        "the restored store must resolve the pointer beside it"
    );
    check!(eq;
        std::fs::read(restored.join("out/publication/current/census.xlsx"))?,
        b"workbook".to_vec(),
        "the restored pointer must resolve to the copied generation"
    );
    let store = Store::open(&restored)?;
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 1);
    Ok(())
}

#[cfg(unix)]
#[test]
fn backup_refuses_a_symlink_that_leaves_the_store() -> TestResult {
    for target in ["../../outside", "/etc/hostname"] {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("store");
        store_with(&root, &["Fenced"])?;
        let link = root.join("out/pointer");
        std::os::unix::fs::symlink(target, &link)?;
        let to = dir.path().join("backup");
        let error = match Store::backup(&root, &to) {
            Err(error) => error.to_string(),
            Ok(report) => {
                return Err(format!("expected a refusal for {target}, got {report:?}").into())
            }
        };
        check!(
            error.contains(&link.display().to_string()),
            "the refusal must name the link: {error}"
        );
        check!(
            error.contains("symlink") && error.contains("inside the store"),
            "the refusal must say what it refused and why: {error}"
        );
        check!(!to.exists(), "a refused backup publishes nothing");
        check!(staging_leftovers(dir.path())?.is_empty());
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn restore_refuses_a_tampered_link_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    store_with_publication(&root)?;
    let to = dir.path().join("backup");
    Store::backup(&root, &to)?;
    let restored = dir.path().join("restored");
    let pointer = "out/publication/current";
    let escaping = "../../../outside";
    edit_manifest_entry(&to, pointer, |entry| {
        entry["target"] = serde_json::json!(escaping);
        entry["sha256"] = serde_json::json!(sha256_hex(escaping.as_bytes()));
    })?;
    let error = refusal_of(Store::restore(&to, &restored))?;
    check!(
        error.contains(pointer) && error.contains("leaves the backup"),
        "an escaping link target must be refused: {error}"
    );
    check!(!restored.exists(), "a refused restore publishes nothing");

    edit_manifest_entry(&to, pointer, |entry| {
        entry["target"] = serde_json::json!("generations/g1");
        entry["sha256"] = serde_json::json!(sha256_hex(b"generations/g1"));
    })?;
    std::fs::remove_file(to.join(pointer))?;
    std::os::unix::fs::symlink("generations/other", to.join(pointer))?;
    let error = refusal_of(Store::restore(&to, &restored))?;
    check!(
        error.contains("symlink target mismatch"),
        "a link whose target moved must be refused: {error}"
    );
    check!(!restored.exists(), "a refused restore publishes nothing");

    std::fs::remove_file(to.join(pointer))?;
    std::fs::write(to.join(pointer), b"generations/g1")?;
    let error = refusal_of(Store::restore(&to, &restored))?;
    check!(
        error.contains("not the symlink the manifest records"),
        "a file standing in for the recorded link must be refused: {error}"
    );
    check!(!restored.exists(), "a refused restore publishes nothing");
    Ok(())
}
