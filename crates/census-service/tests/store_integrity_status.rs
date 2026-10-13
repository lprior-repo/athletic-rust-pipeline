#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use fjall::{Database, KeyspaceCreateOptions, PersistMode};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn create_store(root: &Path) -> TestResult {
    let store = Store::open(root)?;
    let name = "Integrity Fixture School";
    let (school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None);
    store.append(Table::Schools, &school)?;
    store.journal_done(
        "integrity_fixture",
        "school",
        &serde_json::json!({"rows": 1}),
    )?;
    store.flush()?;
    Ok(())
}

fn run_integrity(root: &Path) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(root)
        .arg("store-integrity")
        .output()
}

fn overwrite_first_row(root: &Path, keyspace: &str, bytes: &[u8]) -> TestResult<Vec<u8>> {
    let database = Database::builder(root.join("fjall")).open()?;
    let rows = database.keyspace(keyspace, KeyspaceCreateOptions::default)?;
    let (key, _) = rows
        .iter()
        .next()
        .ok_or("fixture keyspace has no row")?
        .into_inner()?;
    rows.insert(key.as_ref(), bytes)?;
    database.persist(PersistMode::SyncAll)?;
    Ok(key.to_vec())
}

fn read_row(root: &Path, keyspace: &str, key: &[u8]) -> TestResult<Vec<u8>> {
    let database = Database::builder(root.join("fjall")).open()?;
    let rows = database.keyspace(keyspace, KeyspaceCreateOptions::default)?;
    Ok(rows
        .get(key)?
        .ok_or("integrity witness disappeared")?
        .to_vec())
}

fn write_orphan_cache(root: &Path) -> TestResult {
    std::fs::write(root.join("http/orphan.body"), b"retained orphan capture")?;
    Ok(())
}

async fn acquire_local_capture(root: &Path) -> TestResult {
    let body = b"local integrity response";
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/integrity", listener.local_addr()?);
    let fetcher = Fetcher::new(
        root.join("http"),
        None,
        Duration::from_millis(1),
        Default::default(),
        vec!["127.0.0.1".to_string()],
    )?;
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let mut request = [0_u8; 4096];
        if socket.read(&mut request).await? == 0 {
            return Err("local capture received no HTTP request".into());
        }
        let headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(headers.as_bytes()).await?;
        socket.write_all(body).await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let options = FetchOptions::default();
    let (served, fetched) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(server, fetcher.get(&url, &options))
    })
    .await?;
    served?;
    let fetched = fetched?;
    check!(!fetched.from_cache);
    check!(eq; fetched.body, body);
    check!(root.join("http/archive/reservations").is_dir());
    check!(eq; std::fs::read_dir(root.join("http/archive/reservations"))?.count(), 0);
    Ok(())
}

fn create_preserved_cache(root: &Path) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(acquire_local_capture(root))
}

fn cache_witnesses(directory: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?.try_fold(BTreeMap::new(), |mut witnesses, entry| {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            witnesses.extend(cache_witnesses(&entry.path())?);
        } else if kind.is_file() {
            witnesses.insert(entry.path(), std::fs::read(entry.path())?);
        } else {
            return Err("cache witness is not a regular file or directory".into());
        }
        Ok(witnesses)
    })
}

#[test]
fn a_real_preserved_archive_with_empty_reservations_passes_cli_integrity() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    create_preserved_cache(directory.path())?;
    let before = cache_witnesses(&directory.path().join("http"))?;
    let output = run_integrity(directory.path())?;
    check!(eq; output.status.code(), Some(0));
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("ok\ttrue"));
    check!(stdout.contains("http_cache_ok\ttrue"));
    check!(eq; cache_witnesses(&directory.path().join("http"))?, before);
    check!(eq; std::fs::read_dir(directory.path().join("http/archive/reservations"))?.count(), 0);
    Ok(())
}

#[test]
fn unresolved_and_malformed_reservations_fail_cli_integrity_without_repair() -> TestResult {
    let valid_name = format!("{}-{}", "a".repeat(64), "b".repeat(64));
    [
        (valid_name.as_str(), b"23\n45".as_slice()),
        ("malformed", b"invalid record".as_slice()),
    ]
    .into_iter()
    .try_for_each(|(name, bytes)| -> TestResult {
        let directory = tempfile::tempdir()?;
        create_store(directory.path())?;
        create_preserved_cache(directory.path())?;
        std::fs::write(
            directory
                .path()
                .join("http/archive/reservations")
                .join(name),
            bytes,
        )?;
        let before = cache_witnesses(&directory.path().join("http"))?;
        let output = run_integrity(directory.path())?;
        check!(!output.status.success());
        let stdout = std::str::from_utf8(&output.stdout)?;
        check!(stdout.contains("http_cache\tarchive/reservations: unresolved reservations remain"));
        check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=true, http_cache_issues=1"));
        check!(eq; cache_witnesses(&directory.path().join("http"))?, before);
        Ok(())
    })?;
    Ok(())
}

#[test]
fn a_reservations_file_fails_cli_integrity_and_preserves_its_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    create_preserved_cache(directory.path())?;
    let reservations = directory.path().join("http/archive/reservations");
    std::fs::remove_dir(&reservations)?;
    std::fs::write(&reservations, b"not a reservations directory")?;
    let before = cache_witnesses(&directory.path().join("http"))?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    check!(std::str::from_utf8(&output.stdout)?
        .contains("http_cache\tarchive: unexpected entry reservations"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=true, http_cache_issues=1"));
    check!(eq; cache_witnesses(&directory.path().join("http"))?, before);
    Ok(())
}

#[test]
fn clean_existing_store_integrity_exits_zero_with_detailed_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let output = run_integrity(directory.path())?;
    check!(eq; output.status.code(), Some(0));
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("ok\ttrue"));
    check!(stdout.contains("table\tschools\texpected=1\tactual=1\tok"));
    let store = Store::open(directory.path())?;
    check!(eq; store.walk_table(Table::Schools)?.rows, 1);
    check!(store.journal_contains("integrity_fixture", "school")?);
    Ok(())
}

#[test]
fn corrupt_entity_integrity_exits_nonzero_and_preserves_the_row() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let witness = b"{malformed entity";
    let key = overwrite_first_row(directory.path(), "entities", witness)?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    let stderr = std::str::from_utf8(&output.stderr)?;
    check!(stdout.contains("ok\tfalse"));
    check!(stdout.contains("corruption\tUndecodable\trow/schools/"));
    check!(stdout.contains("observation has no string id field"));
    check!(stderr.contains("store_ok=false, http_cache_issues=0"));
    check!(eq; read_row(directory.path(), "entities", &key)?, witness);
    Ok(())
}

#[test]
fn corrupt_journal_integrity_exits_nonzero_and_preserves_the_entry() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let witness = br#"{"type":"done"}"#;
    let key = overwrite_first_row(directory.path(), "journal", witness)?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("corruption\tUndecodable\tjournal/integrity_fixture/school"));
    check!(stdout.contains("the journal entry carries no payload"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=false"));
    check!(eq; read_row(directory.path(), "journal", &key)?, witness);
    Ok(())
}

#[test]
fn counter_mismatch_integrity_exits_nonzero_without_repairing_the_counter() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    {
        let database = Database::builder(directory.path().join("fjall")).open()?;
        let metadata = database.keyspace("meta", KeyspaceCreateOptions::default)?;
        metadata.insert(b"rows:schools", b"9")?;
        database.persist(PersistMode::SyncAll)?;
    }
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("ok\tfalse"));
    check!(stdout.contains("table\tschools\texpected=9\tactual=1\tmismatch"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=false"));
    check!(eq; read_row(directory.path(), "meta", b"rows:schools")?, b"9");
    Ok(())
}

#[test]
fn orphan_cache_integrity_exits_nonzero_and_preserves_capture_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    write_orphan_cache(directory.path())?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("ok\ttrue"));
    check!(stdout.contains("http_cache\thttp: body orphan.body has no metadata"));
    check!(stdout.contains("http_cache_ok\tfalse"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=true, http_cache_issues=1"));
    check!(eq; std::fs::read(directory.path().join("http/orphan.body"))?, b"retained orphan capture");
    Ok(())
}

#[test]
fn digest_mismatched_cache_integrity_exits_nonzero_and_preserves_both_files() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let body = b"retained actual capture";
    let metadata = serde_json::to_vec(&serde_json::json!({
        "content_digest": format!("{:x}", Sha256::digest(b"different claimed capture")),
    }))?;
    let body_path = directory.path().join("http/mismatch.body");
    let metadata_path = directory.path().join("http/mismatch.meta.json");
    std::fs::write(&body_path, body)?;
    std::fs::write(&metadata_path, &metadata)?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("http_cache\t"));
    check!(stdout.contains("digest mismatch"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=true, http_cache_issues=1"));
    check!(eq; std::fs::read(&body_path)?, body);
    check!(eq; std::fs::read(&metadata_path)?, metadata);
    Ok(())
}

#[test]
fn store_and_cache_findings_are_both_reported_before_nonzero_exit() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let witness = b"{malformed entity";
    let key = overwrite_first_row(directory.path(), "entities", witness)?;
    write_orphan_cache(directory.path())?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stdout = std::str::from_utf8(&output.stdout)?;
    check!(stdout.contains("corruption\tUndecodable\trow/schools/"));
    check!(stdout.contains("http_cache\thttp: body orphan.body has no metadata"));
    check!(std::str::from_utf8(&output.stderr)?.contains("store_ok=false, http_cache_issues=1"));
    check!(eq; read_row(directory.path(), "entities", &key)?, witness);
    check!(eq; std::fs::read(directory.path().join("http/orphan.body"))?, b"retained orphan capture");
    Ok(())
}

#[test]
fn store_integrity_read_errors_propagate_without_removing_the_witness() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let path = directory.path().join("journal");
    std::fs::write(&path, b"not a journal directory")?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stderr = std::str::from_utf8(&output.stderr)?;
    check!(stderr.contains("checking store integrity"));
    check!(stderr.contains("journal"));
    check!(eq; std::fs::read(&path)?, b"not a journal directory");
    Ok(())
}

#[test]
fn cache_integrity_read_errors_propagate_without_removing_the_witness() -> TestResult {
    let directory = tempfile::tempdir()?;
    create_store(directory.path())?;
    let path = directory.path().join("http/archive");
    std::fs::write(&path, b"not an archive directory")?;
    let output = run_integrity(directory.path())?;
    check!(!output.status.success());
    let stderr = std::str::from_utf8(&output.stderr)?;
    check!(stderr.contains("checking http cache integrity"));
    check!(stderr.contains("archive"));
    check!(eq; std::fs::read(&path)?, b"not an archive directory");
    Ok(())
}
